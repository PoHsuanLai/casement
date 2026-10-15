//! Capture through a client's dma-buf: the frame is drawn into it with the renderer
//! ([`write_dmabuf`]), and a client's cursor buffer in a dma-buf is read back through the
//! renderer ([`read_dmabuf`]).
//!
//! Both are generic over the renderer; the backend that owns one calls them. The frame is
//! uploaded as a texture and drawn 1:1 into the buffer bound as a framebuffer, so the client's
//! buffer holds exactly the bytes a shared-memory capture would.

use super::{ArgbImage, CaptureFormat, CapturedFrame};
use crate::damage::RenderError;
use casement_core::geometry::PhysicalSize;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::{Buffer as AllocBuffer, Fourcc};
use smithay::backend::renderer::{
    Bind, Color32F, ExportMem, Frame, ImportDma, ImportMem, Renderer, Texture,
};
use smithay::utils::{Buffer, Physical, Rectangle, Size, Transform};

/// Draws `frame` into `target`, a client's dma-buf, with `renderer`: at the buffer's top-left
/// corner, 1:1, over a cleared (transparent) buffer. The buffer may be larger than the frame.
/// Returns once the renderer has finished, so the client may read the buffer.
///
/// ```no_run
/// use casement_render::pass::{CapturedFrame, write_dmabuf};
/// use smithay::backend::allocator::dmabuf::Dmabuf;
/// use smithay::backend::renderer::gles::GlesRenderer;
///
/// # fn demo(renderer: &mut GlesRenderer, buffer: &mut Dmabuf, frame: &CapturedFrame) {
/// let _ = write_dmabuf(renderer, buffer, frame);
/// # }
/// ```
pub fn write_dmabuf<R>(
    renderer: &mut R,
    target: &mut Dmabuf,
    frame: &CapturedFrame,
) -> Result<(), RenderError>
where
    R: Renderer + ImportMem + Bind<Dmabuf>,
{
    let side = |n: u32| i32::try_from(n).map_err(|_| RenderError::NoTarget);
    let fault = |error: &dyn std::fmt::Display| RenderError::Renderer(error.to_string());
    if frame.format != CaptureFormat::Argb8888 || frame.stride != frame.area.w.saturating_mul(4) {
        return Err(RenderError::NoTarget);
    }
    let size = Size::<i32, Buffer>::from((side(frame.area.w)?, side(frame.area.h)?));
    let out = target.size();
    if out.w < size.w || out.h < size.h {
        return Err(RenderError::NoTarget);
    }
    let texture = renderer
        .import_memory(&frame.bytes, Fourcc::Argb8888, size, false)
        .map_err(|e| fault(&e))?;
    let mut framebuffer = renderer.bind(target).map_err(|e| fault(&e))?;
    let whole = Rectangle::<i32, Physical>::from_size(Size::from((out.w, out.h)));
    let mut drawing = renderer
        .render(&mut framebuffer, whole.size, Transform::Normal)
        .map_err(|e| fault(&e))?;
    drawing
        .clear(Color32F::TRANSPARENT, &[whole])
        .map_err(|e| fault(&e))?;
    let shown = Rectangle::<i32, Physical>::from_size(Size::from((size.w, size.h)));
    drawing
        .render_texture_from_to(
            &texture,
            Rectangle::from_size(texture.size()).to_f64(),
            shown,
            &[shown],
            &[],
            Transform::Normal,
            1.0,
        )
        .map_err(|e| fault(&e))?;
    let done = drawing.finish().map_err(|e| fault(&e))?;
    done.wait()
        .map_err(|_| RenderError::Renderer("the dma-buf draw was interrupted".to_owned()))
}

/// Reads `source`, a client's dma-buf (a cursor surface's buffer), through `renderer`.
///
/// ```no_run
/// use casement_render::pass::read_dmabuf;
/// use smithay::backend::allocator::dmabuf::Dmabuf;
/// use smithay::backend::renderer::gles::GlesRenderer;
///
/// # fn demo(renderer: &mut GlesRenderer, buffer: &Dmabuf) {
/// let _ = read_dmabuf(renderer, buffer);
/// # }
/// ```
pub fn read_dmabuf<R>(renderer: &mut R, source: &Dmabuf) -> Result<ArgbImage, RenderError>
where
    R: Renderer + ImportDma + ExportMem,
{
    let fault = |error: &dyn std::fmt::Display| RenderError::Renderer(error.to_string());
    let texture = renderer
        .import_dmabuf(source, None)
        .map_err(|e| fault(&e))?;
    let size = source.size();
    let region = Rectangle::<i32, Buffer>::from_size(size);
    let mapping = renderer
        .copy_texture(&texture, region, Fourcc::Abgr8888)
        .map_err(|e| fault(&e))?;
    let rgba = renderer.map_texture(&mapping).map_err(|e| fault(&e))?;
    let side = |n: i32| u32::try_from(n).map_err(|_| RenderError::NoTarget);
    Ok(ArgbImage::from_rgba(
        PhysicalSize {
            w: side(size.w)?,
            h: side(size.h)?,
        },
        rgba,
    ))
}
