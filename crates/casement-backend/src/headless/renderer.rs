//! The headless renderers: pixman (tier P goldens, byte-exact), llvmpipe GLES on the EGL
//! surfaceless platform (tier L, effects), and smithay's dummy renderer (tests that draw
//! nothing, `testing` only).
//!
//! Each output draws into a persistent offscreen image of its mode's size, so the damage
//! tracker's buffer age is real and an idle frame costs nothing: only a frame with damage is
//! read back (`Abgr8888`, rows top to bottom) into the output's last [`super::RgbaFrame`].

use super::FrameSource;
use crate::backend::BackendError;
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
#[cfg(feature = "testing")]
use smithay::backend::renderer::test::DummyRenderer;
use smithay::backend::renderer::{Bind, ExportMem, Offscreen, RendererSuper};
use smithay::reexports::pixman;
use smithay::utils::{Buffer, Rectangle, Size};
use std::any::Any;
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

/// What a started headless renderer does; one impl per [`SoftRenderer`]. The defaults are the
/// "draws into nothing" answers (the dummy renderer): no capture formats, no dma-buf binding, and
/// every dma-buf verb refused.
pub(super) trait HeadlessRenderer: fmt::Debug {
    /// An offscreen image of `size` this renderer can draw into.
    fn canvas(&mut self, size: PhysicalSize) -> Result<Canvas, BackendError>;

    /// Draws `source`'s elements into `canvas`; the bytes read back when something changed.
    fn draw(
        &mut self,
        canvas: &mut Canvas,
        tracker: &mut OutputDamageTracker,
        source: &dyn FrameSource,
        age: Age,
    ) -> Result<Drawn, BackendError>;

    /// Draws `source`'s elements alone into a fresh image of `size` at `scale` and reads it
    /// back (rows top to bottom, RGBA, transparent where nothing draws): a window's pixels
    /// without whatever overlaps it.
    fn offscreen(
        &mut self,
        size: PhysicalSize,
        scale: Scale,
        source: &dyn FrameSource,
    ) -> Result<Vec<u8>, BackendError>;

    /// The dma-buf formats and modifiers this renderer can draw a capture into: `Argb8888` and
    /// `Xrgb8888` of the set it binds as a render target (pixman: linear only). Empty by default
    /// (the dummy renderer binds nothing).
    fn capture_formats(&self) -> Vec<Format> {
        Vec::new()
    }

    /// Whether this renderer can draw a capture into a dma-buf of format `wanted`: a GL renderer
    /// keeps frames on the GPU and can when it binds the format and modifier; the default
    /// (software) cannot.
    fn binds_format(&self, _wanted: Format) -> bool {
        false
    }

    /// Renders `source` alone straight into the client's dma-buf `target` (window capture on the
    /// GPU: no read-back). Only a GL renderer does; the others refuse and the caller reads back.
    fn render_window_dmabuf(
        &mut self,
        _target: &mut Dmabuf,
        _size: PhysicalSize,
        _scale: Scale,
        _source: &dyn FrameSource,
    ) -> Result<(), BackendError> {
        Err(casement_render::RenderError::NoTarget.into())
    }

    /// Blits `from`, a part of the frame `canvas` holds, into the client's dma-buf `target`: the
    /// frame an output last drew is already a GPU image, so nothing is read back. Only a GL
    /// renderer does.
    fn blit_canvas(
        &mut self,
        _canvas: &Canvas,
        _from: PhysicalRect,
        _target: &mut Dmabuf,
    ) -> Result<(), BackendError> {
        Err(BackendError::CanvasMismatch)
    }

    /// Draws `frame` into the client's dma-buf `target` (a capture's buffer).
    fn write_dmabuf(
        &mut self,
        _target: &mut Dmabuf,
        _frame: &CapturedFrame,
    ) -> Result<(), BackendError> {
        Err(casement_render::RenderError::NoTarget.into())
    }

    /// Reads a client's dma-buf `source` (a cursor surface's buffer) back.
    fn read_dmabuf(&mut self, _source: &Dmabuf) -> Result<ArgbImage, BackendError> {
        Err(casement_render::RenderError::NoTarget.into())
    }
}

impl SoftRenderer {
    /// Starts this renderer (the dummy renderer only under `testing`).
    pub(super) fn start(self) -> Result<Box<dyn HeadlessRenderer>, BackendError> {
        match self {
            #[cfg(feature = "testing")]
            SoftRenderer::Dummy => Ok(Box::<DummyRenderer>::default()),
            #[cfg(not(feature = "testing"))]
            SoftRenderer::Dummy => Err(BackendError::DummyNeedsTesting),
            SoftRenderer::Pixman => PixmanRenderer::new()
                .map(|renderer| Box::new(renderer) as Box<dyn HeadlessRenderer>)
                .map_err(BackendError::from),
            SoftRenderer::Llvmpipe => casement_render::surfaceless_gles()
                .map(|renderer| Box::new(renderer) as Box<dyn HeadlessRenderer>)
                .map_err(BackendError::from),
        }
    }
}

/// smithay's dummy renderer: draws nothing, binds nothing.
#[cfg(feature = "testing")]
impl HeadlessRenderer for DummyRenderer {
    fn canvas(&mut self, size: PhysicalSize) -> Result<Canvas, BackendError> {
        buffer_size(size)?;
        Ok(Canvas::of(size, Box::new(())))
    }

    fn draw(
        &mut self,
        _canvas: &mut Canvas,
        tracker: &mut OutputDamageTracker,
        source: &dyn FrameSource,
        age: Age,
    ) -> Result<Drawn, BackendError> {
        let elements = source.dummy(self);
        let mut framebuffer = smithay::backend::renderer::test::DummyFramebuffer;
        let rendered = render_output(self, &mut framebuffer, tracker, &elements, age.0)
            .map_err(BackendError::from)?;
        Ok(match rendered.damage.is_empty() {
            true => Drawn::Unchanged,
            false => Drawn::Nothing(rendered.damage),
        })
    }

    /// The dummy renderer draws nothing and reads back zeros.
    fn offscreen(
        &mut self,
        size: PhysicalSize,
        _scale: Scale,
        _source: &dyn FrameSource,
    ) -> Result<Vec<u8>, BackendError> {
        buffer_size(size)?;
        let length = usize::try_from(size.w)
            .unwrap_or(0)
            .saturating_mul(usize::try_from(size.h).unwrap_or(0))
            .saturating_mul(4);
        Ok(vec![0; length])
    }
}

impl HeadlessRenderer for PixmanRenderer {
    fn canvas(&mut self, size: PhysicalSize) -> Result<Canvas, BackendError> {
        let buffer = buffer_size(size)?;
        let image: pixman::Image<'static, 'static> = self
            .create_buffer(Fourcc::Abgr8888, buffer)
            .map_err(renderer_fault)?;
        Ok(Canvas::of(size, Box::new(image)))
    }

    fn draw(
        &mut self,
        canvas: &mut Canvas,
        tracker: &mut OutputDamageTracker,
        source: &dyn FrameSource,
        age: Age,
    ) -> Result<Drawn, BackendError> {
        let size = canvas.size;
        let image = canvas
            .target
            .downcast_mut::<pixman::Image<'static, 'static>>()
            .ok_or(BackendError::CanvasMismatch)?;
        let elements = source.pixman(self);
        draw_with(self, image, tracker, &elements, age, size)
    }

    fn offscreen(
        &mut self,
        size: PhysicalSize,
        scale: Scale,
        source: &dyn FrameSource,
    ) -> Result<Vec<u8>, BackendError> {
        let buffer = buffer_size(size)?;
        let mut image = self
            .create_buffer(Fourcc::Abgr8888, buffer)
            .map_err(renderer_fault)?;
        let elements = source.pixman(self);
        render_window(self, &mut image, &elements, size, scale).map_err(BackendError::from)
    }

    fn capture_formats(&self) -> Vec<Format> {
        argb_formats(self)
    }

    fn write_dmabuf(
        &mut self,
        target: &mut Dmabuf,
        frame: &CapturedFrame,
    ) -> Result<(), BackendError> {
        write_dmabuf(self, target, frame).map_err(BackendError::from)
    }

    fn read_dmabuf(&mut self, source: &Dmabuf) -> Result<ArgbImage, BackendError> {
        read_dmabuf(self, source).map_err(BackendError::from)
    }
}

impl HeadlessRenderer for GlesRenderer {
    fn canvas(&mut self, size: PhysicalSize) -> Result<Canvas, BackendError> {
        let buffer = buffer_size(size)?;
        self.create_buffer(Fourcc::Abgr8888, buffer)
            .map(|texture: GlesTexture| Canvas::of(size, Box::new(texture)))
            .map_err(renderer_fault)
    }

    fn draw(
        &mut self,
        canvas: &mut Canvas,
        tracker: &mut OutputDamageTracker,
        source: &dyn FrameSource,
        age: Age,
    ) -> Result<Drawn, BackendError> {
        let size = canvas.size;
        let texture = canvas
            .target
            .downcast_mut::<GlesTexture>()
            .ok_or(BackendError::CanvasMismatch)?;
        let elements = source.gles(self);
        draw_with(self, texture, tracker, &elements, age, size)
    }

    fn offscreen(
        &mut self,
        size: PhysicalSize,
        scale: Scale,
        source: &dyn FrameSource,
    ) -> Result<Vec<u8>, BackendError> {
        let buffer = buffer_size(size)?;
        let mut texture: GlesTexture = self
            .create_buffer(Fourcc::Abgr8888, buffer)
            .map_err(renderer_fault)?;
        let elements = source.gles(self);
        render_window(self, &mut texture, &elements, size, scale).map_err(BackendError::from)
    }

    fn capture_formats(&self) -> Vec<Format> {
        argb_formats(self)
    }

    fn binds_format(&self, wanted: Format) -> bool {
        argb_formats(self).contains(&wanted)
    }

    fn render_window_dmabuf(
        &mut self,
        target: &mut Dmabuf,
        size: PhysicalSize,
        scale: Scale,
        source: &dyn FrameSource,
    ) -> Result<(), BackendError> {
        let elements = source.gles(self);
        render_window_dmabuf(self, target, &elements, size, scale).map_err(BackendError::from)
    }

    fn blit_canvas(
        &mut self,
        canvas: &Canvas,
        from: PhysicalRect,
        target: &mut Dmabuf,
    ) -> Result<(), BackendError> {
        let texture = canvas
            .target
            .downcast_ref::<GlesTexture>()
            .ok_or(BackendError::CanvasMismatch)?;
        blit_texture(self, texture, from, target).map_err(BackendError::from)
    }

    fn write_dmabuf(
        &mut self,
        target: &mut Dmabuf,
        frame: &CapturedFrame,
    ) -> Result<(), BackendError> {
        write_dmabuf(self, target, frame).map_err(BackendError::from)
    }

    fn read_dmabuf(&mut self, source: &Dmabuf) -> Result<ArgbImage, BackendError> {
        read_dmabuf(self, source).map_err(BackendError::from)
    }
}

/// `Argb8888` and `Xrgb8888` of the formats `renderer` binds a dma-buf as.
fn argb_formats<R: Bind<Dmabuf>>(renderer: &R) -> Vec<Format> {
    Bind::<Dmabuf>::supported_formats(renderer)
        .map(|set| {
            set.iter()
                .filter(|format| matches!(format.code, Fourcc::Argb8888 | Fourcc::Xrgb8888))
                .copied()
                .collect()
        })
        .unwrap_or_default()
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
    /// The renderer-specific image (pixman's image, a GLES texture); only its renderer opens it.
    target: Box<dyn Any>,
    /// Whether a frame was drawn into it yet.
    drawn: bool,
}

impl Canvas {
    fn of(size: PhysicalSize, target: Box<dyn Any>) -> Canvas {
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
