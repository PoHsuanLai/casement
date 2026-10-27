//! One window drawn alone (P2-Q3): what `ext_foreign_toplevel_image_capture_source_manager_v1`
//! captures. The window's surface tree is drawn into a fresh offscreen image the size of its
//! content, so whatever overlaps the window on screen, whatever lies off the screen, and a
//! scale that differs from the output's cannot change the pixels. (The other way to make the
//! image, cropping the output's last frame, is wrong under all three; FINDINGS, p2-q3.)
//!
//! The pass is two steps so a backend that owns the renderer can run it: [`window_tree`] builds
//! the elements for its renderer type, [`render_window`] draws them into a target the backend
//! created and reads the pixels back.
//!
//! ```
//! use casement_core::geometry::LogicalSize;
//! use casement_render::pass::window_capture_size;
//! use ds_core::geometry::scale::Scale;
//!
//! let size = window_capture_size(LogicalSize { w: 801, h: 601 }, Scale(150));
//! assert_eq!((size.w, size.h), (1_001, 751));
//! ```

use crate::compositor::Compositor;
use crate::damage::RenderError;
use crate::element::{CasementElement, surface_origin};
use crate::scene::OutputView;
use crate::scene::convert::on_output;
use casement_core::geometry::{
    LogicalPoint, LogicalRect, LogicalSize, PhysicalSize, SubPoint, SubPx, buffer_size,
};
use ds_core::geometry::scale::Scale;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::{Buffer as AllocBuffer, Fourcc};
use smithay::backend::renderer::damage::{Error as DamageError, OutputDamageTracker};
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::render_elements_from_surface_tree;
use smithay::backend::renderer::{Bind, Color32F, ExportMem};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Buffer, Physical, Rectangle, Size, Transform};

/// The size of the image a window with `content` logical pixels is drawn into at `scale`: each
/// side `round(l x s)`, the same conversion every buffer uses.
pub fn window_capture_size(content: LogicalSize, scale: Scale) -> PhysicalSize {
    buffer_size(content, scale)
}

/// The elements of `root`'s surface tree (subsurfaces, then the root), placed so that the
/// window geometry's top-left is the image's top-left, drawn at `scale` and full opacity.
/// `content` is the window geometry's size. Nothing of the screen is in it.
pub fn window_tree<R: Compositor>(
    root: &WlSurface,
    content: LogicalSize,
    scale: Scale,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let view = OutputView {
        area: LogicalRect::new(LogicalPoint { x: 0, y: 0 }, content),
        scale,
        mode: window_capture_size(content, scale),
    };
    let origin = surface_origin(
        root,
        SubPoint {
            x: SubPx(0),
            y: SubPx(0),
        },
    );
    render_elements_from_surface_tree(
        renderer,
        root,
        on_output(origin, &view),
        scale.as_f64(),
        1.0,
        Kind::Unspecified,
    )
}

/// Draws `elements` (front first) into `target`, an image of `size` the caller created in
/// `Abgr8888`, and reads it back: `size.w x size.h` rows of R, G, B, A bytes, top row first.
/// Where nothing draws the image is transparent. Every call draws everything: the tracker is
/// new and the buffer age is 0.
pub fn render_window<R, T>(
    renderer: &mut R,
    target: &mut T,
    elements: &[CasementElement<R>],
    size: PhysicalSize,
    scale: Scale,
) -> Result<Vec<u8>, RenderError>
where
    R: Compositor + Bind<T> + ExportMem,
{
    let side = |n: u32| i32::try_from(n).map_err(|_| RenderError::NoTarget);
    let mode = Size::from((side(size.w)?, side(size.h)?));
    let fault = |error: &dyn std::fmt::Display| RenderError::Renderer(error.to_string());
    let mut tracker = OutputDamageTracker::new(mode, scale.as_f64(), Transform::Normal);
    let elements = crate::damage::routed(renderer, elements);
    let mut framebuffer = renderer.bind(target).map_err(|e| fault(&e))?;
    tracker
        .render_output(
            renderer,
            &mut framebuffer,
            0,
            &elements,
            Color32F::TRANSPARENT,
        )
        .map_err(|error| match error {
            DamageError::Rendering(error) => fault(&error),
            DamageError::OutputNoMode(_) => RenderError::NoTarget,
        })?;
    let region = Rectangle::<i32, Buffer>::from_size(Size::from((mode.w, mode.h)));
    let mapping = renderer
        .copy_framebuffer(&framebuffer, region, Fourcc::Abgr8888)
        .map_err(|e| fault(&e))?;
    let bytes = renderer.map_texture(&mapping).map_err(|e| fault(&e))?;
    Ok(bytes.to_vec())
}

/// Draws `elements` (front first) straight into `target`, a client's dma-buf of exactly `size`,
/// and returns once the renderer has finished: the GPU path of window capture, where no pixel
/// is read back. As [`render_window`]: a new tracker at age 0, transparent where nothing draws.
///
/// ```no_run
/// use casement_core::geometry::PhysicalSize;
/// use casement_render::pass::render_window_dmabuf;
/// use ds_core::geometry::scale::Scale;
/// use smithay::backend::allocator::dmabuf::Dmabuf;
/// use smithay::backend::renderer::gles::GlesRenderer;
///
/// # fn demo(renderer: &mut GlesRenderer, buffer: &mut Dmabuf) {
/// let size = PhysicalSize { w: 800, h: 600 };
/// let _ = render_window_dmabuf(renderer, buffer, &[], size, Scale(120));
/// # }
/// ```
pub fn render_window_dmabuf<R>(
    renderer: &mut R,
    target: &mut Dmabuf,
    elements: &[CasementElement<R>],
    size: PhysicalSize,
    scale: Scale,
) -> Result<(), RenderError>
where
    R: Compositor + Bind<Dmabuf>,
{
    let side = |n: u32| i32::try_from(n).map_err(|_| RenderError::NoTarget);
    let mode = Size::<i32, Physical>::from((side(size.w)?, side(size.h)?));
    let held = target.size();
    if (held.w, held.h) != (mode.w, mode.h) {
        return Err(RenderError::NoTarget);
    }
    let fault = |error: &dyn std::fmt::Display| RenderError::Renderer(error.to_string());
    let mut tracker = OutputDamageTracker::new(mode, scale.as_f64(), Transform::Normal);
    let elements = crate::damage::routed(renderer, elements);
    let mut framebuffer = renderer.bind(target).map_err(|e| fault(&e))?;
    let rendered = tracker
        .render_output(
            renderer,
            &mut framebuffer,
            0,
            &elements,
            Color32F::TRANSPARENT,
        )
        .map_err(|error| match error {
            DamageError::Rendering(error) => fault(&error),
            DamageError::OutputNoMode(_) => RenderError::NoTarget,
        })?;
    rendered
        .sync
        .wait()
        .map_err(|_| RenderError::Renderer("the dma-buf draw was interrupted".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_image_is_the_content_rounded_at_the_scale() {
        let content = LogicalSize { w: 801, h: 601 };
        let size = window_capture_size(content, Scale(180));
        assert_eq!((size.w, size.h), (1_202, 902));
    }
}
