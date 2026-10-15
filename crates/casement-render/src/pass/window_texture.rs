//! One window drawn alone into a GPU image the compositor keeps (wave-2026-10-15/closegpu): the
//! last image of a window that is closing, with no read-back. The elements are drawn into a
//! scratch texture, then once more into the kept texture through the corner mask, so the bottom
//! corners are cut as the window's are and the image can be drawn later as it is.

use crate::damage::RenderError;
use crate::element::{CasementElement, ClipShape, bake_corners};
use casement_core::geometry::{PhysicalRect, PhysicalSize};
use ds_core::geometry::scale::Scale;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::damage::{Error as DamageError, OutputDamageTracker};
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::backend::renderer::{Bind, Color32F, Frame, Offscreen, Renderer};
use smithay::utils::{Buffer, Physical, Rectangle, Size, Transform};

/// Draws `elements` (front first, a window's tree as `window_tree` builds it) into a new texture
/// of `size` at `scale`, transparent where nothing draws, its bottom corners cut by `radius`
/// logical pixels. Returns once the GPU is done; the texture stays on the GPU.
pub fn render_window_texture(
    renderer: &mut GlesRenderer,
    elements: &[CasementElement<GlesRenderer>],
    size: PhysicalSize,
    (scale, radius): (Scale, u32),
) -> Result<GlesTexture, RenderError> {
    let side = |n: u32| i32::try_from(n).map_err(|_| RenderError::NoTarget);
    let (w, h) = (side(size.w)?, side(size.h)?);
    let fault = |error: &dyn std::fmt::Display| RenderError::Renderer(error.to_string());
    let image = |renderer: &mut GlesRenderer| -> Result<GlesTexture, RenderError> {
        Offscreen::<GlesTexture>::create_buffer(
            renderer,
            Fourcc::Abgr8888,
            Size::<i32, Buffer>::from((w, h)),
        )
        .map_err(|e| fault(&e))
    };
    let mut scratch = image(renderer)?;
    draw_elements(renderer, &mut scratch, elements, (w, h), scale)?;
    let mut kept = image(renderer)?;
    let area = PhysicalRect {
        x: 0,
        y: 0,
        w: size.w,
        h: size.h,
    };
    cut_corners(
        renderer,
        &mut kept,
        &scratch,
        ClipShape::new(area, radius, scale),
    )?;
    Ok(kept)
}

/// Draws `elements` into `target`, a fresh tracker at age 0.
fn draw_elements(
    renderer: &mut GlesRenderer,
    target: &mut GlesTexture,
    elements: &[CasementElement<GlesRenderer>],
    (w, h): (i32, i32),
    scale: Scale,
) -> Result<(), RenderError> {
    let fault = |error: &dyn std::fmt::Display| RenderError::Renderer(error.to_string());
    let mode = Size::<i32, Physical>::from((w, h));
    let mut tracker = OutputDamageTracker::new(mode, scale.as_f64(), Transform::Normal);
    let mut framebuffer = renderer.bind(target).map_err(|e| fault(&e))?;
    tracker
        .render_output(
            renderer,
            &mut framebuffer,
            0,
            elements,
            Color32F::TRANSPARENT,
        )
        .map(|_| ())
        .map_err(|error| match error {
            DamageError::Rendering(error) => fault(&error),
            DamageError::OutputNoMode(_) => RenderError::NoTarget,
        })
}

/// Draws `from` into `target` through the corner mask of `shape`, over transparent, and waits.
fn cut_corners(
    renderer: &mut GlesRenderer,
    target: &mut GlesTexture,
    from: &GlesTexture,
    shape: ClipShape,
) -> Result<(), RenderError> {
    let fault = |error: &dyn std::fmt::Display| RenderError::Renderer(error.to_string());
    let rect = shape.rect();
    let side = |n: u32| i32::try_from(n).map_err(|_| RenderError::NoTarget);
    let whole = Rectangle::<i32, Physical>::from_size(Size::from((side(rect.w)?, side(rect.h)?)));
    let mut framebuffer = renderer.bind(target).map_err(|e| fault(&e))?;
    let mut frame = renderer
        .render(&mut framebuffer, whole.size, Transform::Normal)
        .map_err(|e| fault(&e))?;
    frame
        .clear(Color32F::TRANSPARENT, &[whole])
        .map_err(|e| fault(&e))?;
    bake_corners(&mut frame, from, shape).map_err(|e| fault(&e))?;
    let done = frame.finish().map_err(|e| fault(&e))?;
    done.wait()
        .map_err(|_| RenderError::Renderer("the window image draw was interrupted".to_owned()))
}
