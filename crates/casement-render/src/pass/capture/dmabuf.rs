//! Capture through a client's dma-buf. A frame read back earlier is drawn into it with the
//! renderer ([`write_dmabuf`]); a frame that stayed on the GPU is blitted into it
//! ([`blit_texture`], no CPU read); a window is rendered straight into it
//! (`render_window_dmabuf` in `pass/window.rs`); and a client's cursor buffer in a dma-buf is
//! read back through the renderer ([`read_dmabuf`]).
//!
//! All are generic over the renderer; the backend that owns one calls them. Whichever way, the
//! client's buffer holds exactly the bytes a shared-memory capture would.

use super::{ArgbImage, CaptureFormat, CapturedFrame};
use crate::damage::RenderError;
use casement_core::geometry::{PhysicalRect, PhysicalSize};
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::{Buffer as AllocBuffer, Fourcc};
use smithay::backend::renderer::{
    Bind, Color32F, ExportMem, Frame, ImportDma, ImportMem, Renderer, RendererSuper, Texture,
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
    let texture = renderer
        .import_memory(&frame.bytes, Fourcc::Argb8888, size, false)
        .map_err(|e| fault(&e))?;
    let whole = PhysicalRect {
        x: 0,
        y: 0,
        w: frame.area.w,
        h: frame.area.h,
    };
    blit_texture(renderer, &texture, whole, target)
}

/// Draws `from`, a part of `texture` (a frame that stays on the GPU), into `target`, a client's
/// dma-buf: at the buffer's top-left corner, 1:1, over a cleared (transparent) buffer, and
/// returns once the renderer has finished. No byte of the frame is read by the CPU: this is the
/// whole of a direct capture that is not a window (the texture is the output's last frame, drawn
/// by the renderer that draws here). The buffer may be larger than `from`.
///
/// ```no_run
/// use casement_core::geometry::PhysicalRect;
/// use casement_render::pass::blit_texture;
/// use smithay::backend::allocator::dmabuf::Dmabuf;
/// use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
///
/// # fn demo(renderer: &mut GlesRenderer, frame: &GlesTexture, buffer: &mut Dmabuf) {
/// let whole = PhysicalRect { x: 0, y: 0, w: 1920, h: 1080 };
/// let _ = blit_texture(renderer, frame, whole, buffer);
/// # }
/// ```
pub fn blit_texture<R>(
    renderer: &mut R,
    texture: &<R as RendererSuper>::TextureId,
    from: PhysicalRect,
    target: &mut Dmabuf,
) -> Result<(), RenderError>
where
    R: Renderer + Bind<Dmabuf>,
{
    let side = |n: u32| i32::try_from(n).map_err(|_| RenderError::NoTarget);
    let fault = |error: &dyn std::fmt::Display| RenderError::Renderer(error.to_string());
    let (x, y) = (from.x, from.y);
    let size = Size::<i32, Physical>::from((side(from.w)?, side(from.h)?));
    let held = texture.size();
    let out = target.size();
    let inside = x >= 0
        && y >= 0
        && i64::from(x) + i64::from(size.w) <= i64::from(held.w)
        && i64::from(y) + i64::from(size.h) <= i64::from(held.h);
    if out.w < size.w || out.h < size.h || !inside {
        return Err(RenderError::NoTarget);
    }
    let mut framebuffer = renderer.bind(target).map_err(|e| fault(&e))?;
    let whole = Rectangle::<i32, Physical>::from_size(Size::from((out.w, out.h)));
    let mut drawing = renderer
        .render(&mut framebuffer, whole.size, Transform::Normal)
        .map_err(|e| fault(&e))?;
    drawing
        .clear(Color32F::TRANSPARENT, &[whole])
        .map_err(|e| fault(&e))?;
    let shown = Rectangle::<i32, Physical>::from_size(size);
    let source = Rectangle::<i32, Buffer>::new((x, y).into(), (size.w, size.h).into());
    drawing
        .render_texture_from_to(
            texture,
            source.to_f64(),
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
