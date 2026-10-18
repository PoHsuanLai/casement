//! The headless renderers: pixman (tier P goldens, byte-exact), llvmpipe GLES on the EGL
//! surfaceless platform (tier L, effects), and smithay's dummy renderer (tests that draw
//! nothing, `testing` only).
//!
//! Each output draws into a persistent offscreen image of its mode's size, so the damage
//! tracker's buffer age is real and an idle frame costs nothing: only a frame with damage is
//! read back (`Abgr8888`, rows top to bottom) into the output's last [`super::RgbaFrame`].

use super::FrameSource;
use crate::backend::BackendError;
use crate::capture_path::{Import, RendererClass};
use casement_core::geometry::{PhysicalRect, PhysicalSize};
use casement_render::pass::{
    ArgbImage, CapturedFrame, blit_texture, read_dmabuf, render_window, render_window_dmabuf,
    write_dmabuf,
};
use casement_render::{CasementElement, Compositor, render_output};
use ds_core::geometry::scale::Scale;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::{Format, Fourcc, Modifier};
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::backend::renderer::{Bind, ExportMem, Offscreen, RendererSuper};
use smithay::reexports::pixman;
use smithay::utils::{Buffer, Rectangle, Size};
use std::fmt;

/// Which software renderer the headless backend uses (`--renderer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoftRenderer {
    /// smithay's dummy renderer: nothing is drawn (`testing` only).
    Dummy,
    /// pixman: exact, no effects.
    Pixman,
    /// GLES on Mesa's llvmpipe: effects run.
    Llvmpipe,
}

/// A started headless renderer.
#[derive(Debug)]
pub enum HeadlessRenderer {
    /// smithay's dummy renderer.
    #[cfg(feature = "testing")]
    Dummy(Box<smithay::backend::renderer::test::DummyRenderer>),
    /// pixman.
    Pixman(Box<PixmanRenderer>),
    /// GLES on llvmpipe (`casement_render::surfaceless_gles`, the one unsafe module).
    Gles(Box<GlesRenderer>),
}

impl HeadlessRenderer {
    /// Starts `choice` (the dummy renderer only under `testing`).
    pub fn start(choice: SoftRenderer) -> Result<HeadlessRenderer, BackendError> {
        match choice {
            #[cfg(feature = "testing")]
            SoftRenderer::Dummy => Ok(HeadlessRenderer::Dummy(Box::default())),
            #[cfg(not(feature = "testing"))]
            SoftRenderer::Dummy => Err(BackendError::DummyNeedsTesting),
            SoftRenderer::Pixman => PixmanRenderer::new()
                .map(|renderer| HeadlessRenderer::Pixman(Box::new(renderer)))
                .map_err(BackendError::from),
            SoftRenderer::Llvmpipe => casement_render::surfaceless_gles()
                .map(|renderer| HeadlessRenderer::Gles(Box::new(renderer)))
                .map_err(BackendError::from),
        }
    }

    /// An offscreen image of `size` this renderer can draw into.
    pub(super) fn canvas(&mut self, size: PhysicalSize) -> Result<Canvas, BackendError> {
        let buffer = buffer_size(size)?;
        match self {
            #[cfg(feature = "testing")]
            HeadlessRenderer::Dummy(_) => Ok(Canvas::of(size, Target::Dummy)),
            HeadlessRenderer::Pixman(renderer) => {
                let image = renderer
                    .create_buffer(Fourcc::Abgr8888, buffer)
                    .map_err(renderer_fault)?;
                Ok(Canvas::of(size, Target::Pixman(image)))
            }
            HeadlessRenderer::Gles(renderer) => renderer
                .create_buffer(Fourcc::Abgr8888, buffer)
                .map(|texture: GlesTexture| Canvas::of(size, Target::Gles(texture)))
                .map_err(renderer_fault),
        }
    }

    /// Draws `source`'s elements into `canvas`; the bytes read back when something changed.
    pub(super) fn draw(
        &mut self,
        canvas: &mut Canvas,
        tracker: &mut OutputDamageTracker,
        source: &dyn FrameSource,
        age: Age,
    ) -> Result<Drawn, BackendError> {
        let size = canvas.size;
        match (self, &mut canvas.target) {
            #[cfg(feature = "testing")]
            (HeadlessRenderer::Dummy(renderer), Target::Dummy) => {
                let renderer = renderer.as_mut();
                let elements = source.dummy(renderer);
                let mut framebuffer = smithay::backend::renderer::test::DummyFramebuffer;
                let rendered = render_output(renderer, &mut framebuffer, tracker, &elements, age.0)
                    .map_err(BackendError::from)?;
                Ok(match rendered.damage.is_empty() {
                    true => Drawn::Unchanged,
                    false => Drawn::Nothing(rendered.damage),
                })
            }
            (HeadlessRenderer::Pixman(renderer), Target::Pixman(image)) => {
                let renderer = renderer.as_mut();
                let elements = source.pixman(renderer);
                draw_with(renderer, image, tracker, &elements, age, size)
            }
            (HeadlessRenderer::Gles(renderer), Target::Gles(texture)) => {
                let renderer = renderer.as_mut();
                let elements = source.gles(renderer);
                draw_with(renderer, texture, tracker, &elements, age, size)
            }
            _ => Err(BackendError::CanvasMismatch),
        }
    }
}

impl HeadlessRenderer {
    /// The dma-buf formats and modifiers this renderer can draw a capture into: `Argb8888` and
    /// `Xrgb8888` of the set it binds as a render target (pixman: linear only). Empty for the
    /// dummy renderer, which binds nothing.
    pub fn capture_formats(&self) -> Vec<Format> {
        let set = match self {
            #[cfg(feature = "testing")]
            HeadlessRenderer::Dummy(_) => None,
            HeadlessRenderer::Pixman(renderer) => Bind::<Dmabuf>::supported_formats(&**renderer),
            HeadlessRenderer::Gles(renderer) => Bind::<Dmabuf>::supported_formats(&**renderer),
        };
        set.map(|set| {
            set.iter()
                .filter(|format| matches!(format.code, Fourcc::Argb8888 | Fourcc::Xrgb8888))
                .copied()
                .collect()
        })
        .unwrap_or_default()
    }

    /// What this renderer is to a capture into a dma-buf of format `wanted`: a GL renderer
    /// keeps frames on the GPU and can draw into the buffer when it binds the format and
    /// modifier; pixman and the dummy renderer are software.
    pub fn capture_class(&self, wanted: Format) -> RendererClass {
        match self {
            #[cfg(feature = "testing")]
            HeadlessRenderer::Dummy(_) => RendererClass::Software,
            HeadlessRenderer::Pixman(_) => RendererClass::Software,
            HeadlessRenderer::Gles(_) => {
                RendererClass::Gpu(match self.capture_formats().contains(&wanted) {
                    true => Import::Accepted,
                    false => Import::Refused,
                })
            }
        }
    }

    /// Renders `source` alone straight into the client's dma-buf `target` (window capture on the
    /// GPU: no read-back). Only a GL renderer does; the others refuse and the caller reads back.
    pub(super) fn render_window_dmabuf(
        &mut self,
        target: &mut Dmabuf,
        size: PhysicalSize,
        scale: Scale,
        source: &dyn FrameSource,
    ) -> Result<(), BackendError> {
        match self {
            HeadlessRenderer::Gles(renderer) => {
                let renderer = renderer.as_mut();
                let elements = source.gles(renderer);
                render_window_dmabuf(renderer, target, &elements, size, scale)
                    .map_err(BackendError::from)
            }
            _ => Err(casement_render::RenderError::NoTarget.into()),
        }
    }

    /// Blits `from`, a part of the frame `canvas` holds, into the client's dma-buf `target`: the
    /// frame an output last drew is already a GPU image, so nothing is read back.
    pub(super) fn blit_canvas(
        &mut self,
        canvas: &Canvas,
        from: PhysicalRect,
        target: &mut Dmabuf,
    ) -> Result<(), BackendError> {
        match (self, &canvas.target) {
            (HeadlessRenderer::Gles(renderer), Target::Gles(texture)) => {
                blit_texture(renderer.as_mut(), texture, from, target).map_err(BackendError::from)
            }
            _ => Err(BackendError::CanvasMismatch),
        }
    }

    /// Draws `frame` into the client's dma-buf `target` (a capture's buffer).
    pub fn write_dmabuf(
        &mut self,
        target: &mut Dmabuf,
        frame: &CapturedFrame,
    ) -> Result<(), BackendError> {
        match self {
            #[cfg(feature = "testing")]
            HeadlessRenderer::Dummy(_) => Err(casement_render::RenderError::NoTarget.into()),
            HeadlessRenderer::Pixman(renderer) => {
                write_dmabuf(renderer.as_mut(), target, frame).map_err(BackendError::from)
            }
            HeadlessRenderer::Gles(renderer) => {
                write_dmabuf(renderer.as_mut(), target, frame).map_err(BackendError::from)
            }
        }
    }

    /// Reads a client's dma-buf `source` (a cursor surface's buffer) back.
    pub fn read_dmabuf(&mut self, source: &Dmabuf) -> Result<ArgbImage, BackendError> {
        match self {
            #[cfg(feature = "testing")]
            HeadlessRenderer::Dummy(_) => Err(casement_render::RenderError::NoTarget.into()),
            HeadlessRenderer::Pixman(renderer) => {
                read_dmabuf(renderer.as_mut(), source).map_err(BackendError::from)
            }
            HeadlessRenderer::Gles(renderer) => {
                read_dmabuf(renderer.as_mut(), source).map_err(BackendError::from)
            }
        }
    }
}

/// The formats a software renderer draws a capture into when it has not been started: linear
/// `Argb8888` and `Xrgb8888`, what pixman binds.
pub fn linear_capture_formats() -> Vec<Format> {
    [Fourcc::Argb8888, Fourcc::Xrgb8888]
        .into_iter()
        .map(|code| Format {
            code,
            modifier: Modifier::Linear,
        })
        .collect()
}

impl HeadlessRenderer {
    /// Draws `source`'s elements alone into a fresh image of `size` at `scale` and reads it
    /// back (rows top to bottom, RGBA, transparent where nothing draws): a window's pixels
    /// without whatever overlaps it. The dummy renderer draws nothing and reads back zeros.
    pub(super) fn offscreen(
        &mut self,
        size: PhysicalSize,
        scale: Scale,
        source: &dyn FrameSource,
    ) -> Result<Vec<u8>, BackendError> {
        let buffer = buffer_size(size)?;
        match self {
            #[cfg(feature = "testing")]
            HeadlessRenderer::Dummy(_) => {
                let length = usize::try_from(size.w)
                    .unwrap_or(0)
                    .saturating_mul(usize::try_from(size.h).unwrap_or(0))
                    .saturating_mul(4);
                Ok(vec![0; length])
            }
            HeadlessRenderer::Pixman(renderer) => {
                let renderer = renderer.as_mut();
                let mut image = renderer
                    .create_buffer(Fourcc::Abgr8888, buffer)
                    .map_err(renderer_fault)?;
                let elements = source.pixman(renderer);
                render_window(renderer, &mut image, &elements, size, scale)
                    .map_err(BackendError::from)
            }
            HeadlessRenderer::Gles(renderer) => {
                let renderer = renderer.as_mut();
                let mut texture: GlesTexture = renderer
                    .create_buffer(Fourcc::Abgr8888, buffer)
                    .map_err(renderer_fault)?;
                let elements = source.gles(renderer);
                render_window(renderer, &mut texture, &elements, size, scale)
                    .map_err(BackendError::from)
            }
        }
    }
}

/// The buffer age of a canvas: 0 before its first frame (everything is redrawn), else 1 (it
/// holds the previous frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Age(pub usize);

/// What drawing a frame produced.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Drawn {
    /// The frame, read back (rows top to bottom, RGBA), and the area that changed since the
    /// last one, in the frame's pixels.
    Pixels(Vec<u8>, Vec<PhysicalRect>),
    /// The dummy renderer drew something and has nothing to read back; the area it drew.
    Nothing(Vec<PhysicalRect>),
    /// Nothing was damaged: the canvas still holds the last frame.
    Unchanged,
}

/// One output's offscreen image and its size.
pub struct Canvas {
    size: PhysicalSize,
    target: Target,
    /// Whether a frame was drawn into it yet.
    drawn: bool,
}

impl Canvas {
    fn of(size: PhysicalSize, target: Target) -> Canvas {
        Canvas {
            size,
            target,
            drawn: false,
        }
    }

    /// The age the next frame draws with.
    pub(super) fn age(&self) -> Age {
        Age(usize::from(self.drawn))
    }

    /// The canvas has had a frame drawn into it.
    pub(super) fn mark_drawn(&mut self) {
        self.drawn = true;
    }
}

impl fmt::Debug for Canvas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Canvas({}x{})", self.size.w, self.size.h)
    }
}

/// The renderer-specific image.
enum Target {
    #[cfg(feature = "testing")]
    Dummy,
    Pixman(pixman::Image<'static, 'static>),
    Gles(GlesTexture),
}

/// Binds `target`, renders `elements` and reads the frame back when it had damage.
fn draw_with<R, T>(
    renderer: &mut R,
    target: &mut T,
    tracker: &mut OutputDamageTracker,
    elements: &[CasementElement<R>],
    age: Age,
    size: PhysicalSize,
) -> Result<Drawn, BackendError>
where
    R: Compositor + Bind<T> + ExportMem,
    <R as RendererSuper>::Error: Into<BackendError>,
    <R as RendererSuper>::TextureId: 'static,
{
    let mut framebuffer = renderer.bind(target).map_err(renderer_fault)?;
    let rendered = render_output(renderer, &mut framebuffer, tracker, elements, age.0)
        .map_err(BackendError::from)?;
    if rendered.damage.is_empty() {
        return Ok(Drawn::Unchanged);
    }
    let region = Rectangle::from_size(buffer_size(size)?);
    let mapping = renderer
        .copy_framebuffer(&framebuffer, region, Fourcc::Abgr8888)
        .map_err(renderer_fault)?;
    let bytes = renderer.map_texture(&mapping).map_err(renderer_fault)?;
    Ok(Drawn::Pixels(rows_top_down(bytes, size), rendered.damage))
}

/// `bytes` as `size` rows of RGBA, top row first. The rows of a read-back come top first on
/// both renderers (GLES says its mapping is `flipped`, but the damage tracker's framebuffer
/// is already y-inverted, so reversing the rows would turn the frame upside down; pixman and
/// llvmpipe agree byte for byte on aligned scenes, tier X).
fn rows_top_down(bytes: &[u8], size: PhysicalSize) -> Vec<u8> {
    let width = usize::try_from(size.w).unwrap_or(0).saturating_mul(4);
    let height = usize::try_from(size.h).unwrap_or(0);
    let mut out = Vec::with_capacity(width.saturating_mul(height));
    bytes
        .chunks(width.max(1))
        .take(height)
        .for_each(|row| out.extend_from_slice(row));
    out
}

/// `size` as smithay's buffer size.
fn buffer_size(size: PhysicalSize) -> Result<Size<i32, Buffer>, BackendError> {
    let fits = |n: u32| i32::try_from(n).ok();
    fits(size.w)
        .zip(fits(size.h))
        .map(|(w, h)| Size::from((w, h)))
        .ok_or(BackendError::OutputTooLarge {
            w: size.w,
            h: size.h,
        })
}

/// A renderer's own error as ours.
fn renderer_fault(error: impl Into<BackendError>) -> BackendError {
    error.into()
}
