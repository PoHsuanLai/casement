//! The copy route's picture (feature `tty`): an output scanned out by a GPU that does not render
//! is drawn on the render GPU into a dma-buf the scanout GPU allocated, and the scanout GPU's
//! own compositor puts that one texture on the glass. A thin adapter (B7); the choice of format
//! is `casement_core::gpu::bridge_format` and the choice of route is the render plan's.
//!
//! One buffer serves every frame. That is safe because a surface queues nothing while a flip is
//! pending (`QueueOutcome::Busy`): the scanout GPU has finished sampling the picture before the
//! render GPU draws the next one. The render GPU's damage tracker therefore always sees the
//! same buffer (age 1) and says nothing changed when nothing did, so a still desktop costs
//! neither GPU a frame. The render GPU is waited for on the CPU before the scanout GPU samples
//! (no implicit fence crosses vendors). The hardware cursor plane is not used on this route:
//! the pointer is drawn into the picture.

use super::kms::KmsError;
use super::surface_real::{CLEAR, Compositor, refused};
use crate::headless::FrameSource;
use casement_core::gpu::{
    FormatEntry, Fourcc as CoreFourcc, Modifier as CoreModifier, bridge_format,
};
use smithay::backend::allocator::dmabuf::{AsDmabuf, Dmabuf};
use smithay::backend::allocator::gbm::{GbmAllocator, GbmBufferFlags};
use smithay::backend::allocator::{Allocator, Format, Fourcc, Modifier};
use smithay::backend::drm::DrmDeviceFd;
use smithay::backend::drm::compositor::FrameFlags;
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::texture::{TextureRenderBuffer, TextureRenderElement};
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::backend::renderer::{Bind, ImportDma};
use smithay::reexports::gbm::Device as GbmDevice;
use smithay::utils::{Buffer, Physical, Rectangle, Size, Transform};
use std::cell::RefCell;
use std::convert::Infallible;
use std::rc::Rc;

/// The formats the picture may take, best first: opaque, so the scanout GPU blends nothing.
const WANTED: [CoreFourcc; 2] = [CoreFourcc(0x3432_5258), CoreFourcc(0x3432_5241)];

/// Whether the picture holds the previous frame, which the damage tracker may build on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Held {
    /// It holds nothing usable (new, or the screen was lost): draw everything.
    Nothing,
    /// It holds the last frame drawn.
    LastFrame,
}

/// What drawing a frame on this route did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Drawn {
    /// Nothing changed since the last frame.
    Unchanged,
    /// The scanout GPU's compositor holds a new frame to queue.
    Frame,
}

/// One output's copy route.
pub(super) struct Bridge {
    render: Rc<RefCell<GlesRenderer>>,
    local: Rc<RefCell<GlesRenderer>>,
    picture: Dmabuf,
    tracker: OutputDamageTracker,
    shown: TextureRenderBuffer<GlesTexture>,
    held: Held,
}

impl Bridge {
    /// The route for an output of `mode` pixels at `scale` and `transform`: `render` draws,
    /// `local` (the scanout GPU's renderer) samples the picture, which `gbm` allocates. Fails,
    /// with the reason in plain words, when the two GPUs share no buffer format or a step of
    /// the setup is refused; the caller then draws with the scanout GPU alone.
    pub(super) fn open(
        (render, local): (Rc<RefCell<GlesRenderer>>, Rc<RefCell<GlesRenderer>>),
        gbm: &GbmDevice<DrmDeviceFd>,
        (mode, scale, transform): (Size<i32, Physical>, f64, Transform),
    ) -> Result<Bridge, KmsError> {
        let entry = {
            let drawn = entries(render.borrow().egl_context().dmabuf_render_formats().iter());
            let sampled = entries(local.borrow().dmabuf_formats().iter());
            bridge_format(&drawn, &sampled, &WANTED).ok_or_else(|| {
                refused("the two GPUs share no picture format the render GPU can draw to")
            })?
        };
        let format = Fourcc::try_from(entry.format.0)
            .map_err(|_| refused("the picture format is not a known format"))?;
        let modifier = Modifier::from(entry.modifier.0);
        let width = u32::try_from(mode.w).map_err(|_| refused("the mode is negative"))?;
        let height = u32::try_from(mode.h).map_err(|_| refused("the mode is negative"))?;
        let mut allocator = GbmAllocator::new(gbm.clone(), GbmBufferFlags::RENDERING);
        let buffer = allocator
            .create_buffer(width, height, format, &[modifier])
            .map_err(refused)?;
        let picture = buffer.export().map_err(refused)?;
        let size = Size::<i32, Buffer>::from((mode.w, mode.h));
        let texture = local
            .borrow_mut()
            .import_dmabuf(&picture, None)
            .map_err(|error| refused(format!("the scanout GPU cannot sample it: {error}")))?;
        // The picture is always opaque (the frame clears to opaque black).
        let opaque = Some(vec![Rectangle::from_size(size)]);
        let shown = TextureRenderBuffer::from_texture(
            &*local.borrow(),
            texture,
            1,
            Transform::Normal,
            opaque,
        );
        // Prove the render GPU can bind it now, so a refusal is a fallback and not a dark screen.
        let mut probe = picture.clone();
        render
            .borrow_mut()
            .bind(&mut probe)
            .map(drop)
            .map_err(|error| refused(format!("the render GPU cannot draw into it: {error}")))?;
        Ok(Bridge {
            render,
            local,
            picture,
            tracker: OutputDamageTracker::new(mode, scale, transform),
            shown,
            held: Held::Nothing,
        })
    }

    /// The scanout GPU's renderer, which composes the picture.
    pub(super) fn local(&self) -> &Rc<RefCell<GlesRenderer>> {
        &self.local
    }

    /// The screen was lost (dark, or the seat came back): the next frame draws everything.
    pub(super) fn forget(&mut self) {
        self.held = Held::Nothing;
    }

    /// Draws `source` on the render GPU into the picture and, when anything changed, makes the
    /// scanout GPU's `compositor` compose the picture.
    pub(super) fn draw(
        &mut self,
        compositor: &mut Compositor,
        source: &dyn FrameSource,
    ) -> Result<Drawn, KmsError> {
        if self.paint(source)? == Drawn::Unchanged {
            return Ok(Drawn::Unchanged);
        }
        let elements = [self.element()];
        let mut local = self.local.borrow_mut();
        let result = compositor
            .render_frame(&mut *local, &elements, CLEAR, FrameFlags::DEFAULT)
            .map_err(refused)?;
        Ok(match result.is_empty {
            true => Drawn::Unchanged,
            false => Drawn::Frame,
        })
    }

    /// Draws the frame into the picture and waits for the render GPU to finish.
    fn paint(&mut self, source: &dyn FrameSource) -> Result<Drawn, KmsError> {
        let age = match self.held {
            Held::Nothing => 0,
            Held::LastFrame => 1,
        };
        let mut render = self.render.borrow_mut();
        let elements = source.gles(&mut render);
        let mut framebuffer = render.bind(&mut self.picture).map_err(refused)?;
        let rendered = self
            .tracker
            .render_output(&mut *render, &mut framebuffer, age, &elements, CLEAR)
            .map_err(refused)?;
        let changed = rendered.damage.is_some();
        rendered
            .sync
            .wait()
            .map_err(|_| refused("the render GPU was interrupted"))?;
        self.held = Held::LastFrame;
        Ok(match changed {
            true => Drawn::Frame,
            false => Drawn::Unchanged,
        })
    }

    /// The picture as the one element the scanout compositor draws, all of it changed.
    fn element(&mut self) -> TextureRenderElement<GlesTexture> {
        let size = self.picture_size();
        let damage = vec![Rectangle::from_size(size)];
        let _ = self.shown.render().draw(|_| Ok::<_, Infallible>(damage));
        TextureRenderElement::from_texture_render_buffer(
            (0.0, 0.0),
            &self.shown,
            None,
            None,
            None,
            Kind::Unspecified,
        )
    }

    fn picture_size(&self) -> Size<i32, Buffer> {
        use smithay::backend::allocator::Buffer as _;
        self.picture.size()
    }
}

/// `formats` as the plan's pure entries.
fn entries<'a>(formats: impl Iterator<Item = &'a Format>) -> Vec<FormatEntry> {
    formats
        .map(|format| FormatEntry {
            format: CoreFourcc(format.code as u32),
            modifier: CoreModifier(u64::from(format.modifier)),
        })
        .collect()
}

impl std::fmt::Debug for Bridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bridge")
            .field("held", &self.held)
            .finish_non_exhaustive()
    }
}
