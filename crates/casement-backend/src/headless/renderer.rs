//! The headless renderers: pixman (tier P goldens, byte-exact), llvmpipe GLES on the EGL
//! surfaceless platform (tier L, effects), and smithay's dummy renderer (tests that draw
//! nothing, `testing` only).
//!
//! Each output draws into a persistent offscreen image of its mode's size, so the damage
//! tracker's buffer age is real and an idle frame costs nothing: only a frame with damage is
//! read back (`Abgr8888`, rows top to bottom) into the output's last [`super::RgbaFrame`].

use super::FrameSource;
use crate::backend::BackendError;
use casement_core::geometry::PhysicalSize;
use casement_render::{CasementElement, Compositor, render_output};
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::backend::renderer::{Bind, ExportMem, Offscreen, RendererSuper};
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
            SoftRenderer::Dummy => Err(BackendError::Renderer(
                "the dummy renderer needs the `testing` feature".to_owned(),
            )),
            SoftRenderer::Pixman => PixmanRenderer::new()
                .map(|renderer| HeadlessRenderer::Pixman(Box::new(renderer)))
                .map_err(|error| BackendError::Renderer(error.to_string())),
            SoftRenderer::Llvmpipe => casement_render::surfaceless_gles()
                .map(|renderer| HeadlessRenderer::Gles(Box::new(renderer)))
                .map_err(|error| BackendError::Renderer(error.to_string())),
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
                Ok(Canvas::of(size, Target::Pixman(Box::new(Held(image)))))
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
                    .map_err(|error| BackendError::Renderer(error.to_string()))?;
                Ok(match rendered.damage.is_empty() {
                    true => Drawn::Unchanged,
                    false => Drawn::Nothing,
                })
            }
            (HeadlessRenderer::Pixman(renderer), Target::Pixman(held)) => {
                held.draw(renderer.as_mut(), tracker, source, age, size)
            }
            (HeadlessRenderer::Gles(renderer), Target::Gles(texture)) => {
                let renderer = renderer.as_mut();
                let elements = source.gles(renderer);
                draw_with(renderer, texture, tracker, &elements, age, size)
            }
            _ => Err(BackendError::Renderer(
                "an output's canvas belongs to another renderer".to_owned(),
            )),
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
    /// The frame, read back (rows top to bottom, RGBA).
    Pixels(Vec<u8>),
    /// The dummy renderer drew something and has nothing to read back.
    Nothing,
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
    Pixman(Box<dyn PixmanTarget>),
    Gles(GlesTexture),
}

/// pixman's image type is private to smithay (and to the `pixman` crate we do not depend on),
/// so the image is held behind this trait, typed by inference at its one creation site.
trait PixmanTarget {
    fn draw(
        &mut self,
        renderer: &mut PixmanRenderer,
        tracker: &mut OutputDamageTracker,
        source: &dyn FrameSource,
        age: Age,
        size: PhysicalSize,
    ) -> Result<Drawn, BackendError>;
}

/// A pixman image of whatever type smithay binds.
struct Held<T>(T);

impl<T> PixmanTarget for Held<T>
where
    PixmanRenderer: Bind<T>,
{
    fn draw(
        &mut self,
        renderer: &mut PixmanRenderer,
        tracker: &mut OutputDamageTracker,
        source: &dyn FrameSource,
        age: Age,
        size: PhysicalSize,
    ) -> Result<Drawn, BackendError> {
        let elements = source.pixman(renderer);
        draw_with(renderer, &mut self.0, tracker, &elements, age, size)
    }
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
    <R as RendererSuper>::Error: fmt::Display,
    <R as RendererSuper>::TextureId: 'static,
{
    let mut framebuffer = renderer.bind(target).map_err(renderer_fault)?;
    let rendered = render_output(renderer, &mut framebuffer, tracker, elements, age.0)
        .map_err(|error| BackendError::Renderer(error.to_string()))?;
    if rendered.damage.is_empty() {
        return Ok(Drawn::Unchanged);
    }
    let region = Rectangle::from_size(buffer_size(size)?);
    let mapping = renderer
        .copy_framebuffer(&framebuffer, region, Fourcc::Abgr8888)
        .map_err(renderer_fault)?;
    let bytes = renderer.map_texture(&mapping).map_err(renderer_fault)?;
    Ok(Drawn::Pixels(rows_top_down(bytes, size)))
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
        .ok_or_else(|| {
            BackendError::Renderer(format!("a {}x{} output is too large", size.w, size.h))
        })
}

fn renderer_fault(error: impl fmt::Display) -> BackendError {
    BackendError::Renderer(error.to_string())
}
