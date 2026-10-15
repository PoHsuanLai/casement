//! A server-side frame as elements: the titlebar with its lights in front of the window's
//! surfaces, the shadow behind them. Both are casement's own rasters (`frame/raster.rs`),
//! cached in the [`TextureBook`] per window, size, scale and look.

mod raster;
mod title;

pub use title::TitleDraw;

use super::CasementElement;
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::scene::convert::{on_output, sub_of};
use crate::texture_book::TextureBook;
use casement_core::WindowId;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, buffer_size};
use casement_core::window::FrameStyle;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::memory::{
    MemoryRenderBuffer, MemoryRenderBufferRenderElement,
};
use smithay::utils::{Logical, Rectangle, Size, Transform};

/// The frame `style` makes round the window `window` whose content is at `content`, on `view`.
/// Nothing for the part that is off this output. The first list goes in front of the window's
/// surfaces (the title, then the titlebar and the lights), the second behind them (the shadow).
/// `title`, when there is one, is laid over the titlebar.
pub fn frame_elements<R: Compositor>(
    window: WindowId,
    style: &FrameStyle,
    title: Option<&TitleDraw>,
    content: LogicalRect,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> (Vec<CasementElement<R>>, Vec<CasementElement<R>>) {
    let geometry = style.around(content);
    let bar = geometry.titlebar();
    let outline = geometry.outline();
    let front: Vec<CasementElement<R>> = view
        .area
        .intersection(&bar)
        .and_then(|_| {
            let buffer = book.cache().frame(
                (
                    window,
                    0,
                    view.scale.numerator(),
                    bar.size.w,
                    bar.size.h,
                    style.look,
                    0,
                ),
                || {
                    let made =
                        raster::titlebar(&style.look, &style.metrics, bar.size.w, view.scale);
                    to_buffer(&made)
                },
            );
            let pixels = buffer_size(bar.size, view.scale);
            element(
                renderer,
                view,
                bar.origin,
                bar.size,
                (pixels.w, pixels.h),
                &buffer,
            )
        })
        .map(CasementElement::Memory)
        .into_iter()
        .collect();
    let front = title_element(window, style, title, &bar, view, book, renderer)
        .into_iter()
        .chain(front)
        .collect();
    let margin = i32::try_from(raster::SHADOW_MARGIN).unwrap_or(0);
    let reach = LogicalRect::new(
        LogicalPoint {
            x: outline.origin.x.saturating_sub(margin),
            y: outline.origin.y.saturating_sub(margin),
        },
        LogicalSize {
            w: outline.size.w + 2 * raster::SHADOW_MARGIN,
            h: outline.size.h + 2 * raster::SHADOW_MARGIN,
        },
    );
    let back = view
        .area
        .intersection(&reach)
        .and_then(|_| {
            let buffer = book.cache().frame(
                (
                    window,
                    1,
                    120,
                    outline.size.w,
                    outline.size.h,
                    style.look,
                    0,
                ),
                || to_buffer(&raster::shadow(&style.look, outline.size.w, outline.size.h)),
            );
            element(
                renderer,
                view,
                reach.origin,
                reach.size,
                (reach.size.w, reach.size.h),
                &buffer,
            )
        })
        .map(CasementElement::Memory)
        .into_iter()
        .collect();
    (front, back)
}

/// The title laid over the titlebar `bar`: the same size and place as the titlebar's raster, so
/// the pixels line up one to one. Nothing for no title, a title that is not that size, or a bar
/// off this output.
fn title_element<R: Compositor>(
    window: WindowId,
    style: &FrameStyle,
    title: Option<&TitleDraw>,
    bar: &LogicalRect,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Option<CasementElement<R>> {
    let title = title?;
    view.area.intersection(bar)?;
    let pixels = buffer_size(bar.size, view.scale);
    if (title.width, title.height) != (pixels.w, pixels.h) {
        return None;
    }
    let argb = title.argb()?;
    let buffer = book.cache().frame(
        (
            window,
            2,
            view.scale.numerator(),
            bar.size.w,
            bar.size.h,
            style.look,
            title.id,
        ),
        || {
            to_buffer(&raster::Raster {
                w: title.width,
                h: title.height,
                argb,
            })
        },
    );
    element(
        renderer,
        view,
        bar.origin,
        bar.size,
        (pixels.w, pixels.h),
        &buffer,
    )
    .map(CasementElement::Memory)
}

/// `made` as a buffer smithay can upload.
fn to_buffer(made: &raster::Raster) -> MemoryRenderBuffer {
    MemoryRenderBuffer::from_slice(
        &made.argb,
        Fourcc::Argb8888,
        (
            i32::try_from(made.w).unwrap_or(i32::MAX),
            i32::try_from(made.h).unwrap_or(i32::MAX),
        ),
        1,
        Transform::Normal,
        None,
    )
}

/// `buffer` drawn with its top-left at the global logical `origin`, stretched to `size`
/// logical pixels (the buffer is native for a titlebar and one pixel to a logical pixel for a
/// shadow).
fn element<R: Compositor>(
    renderer: &mut R,
    view: &OutputView,
    origin: LogicalPoint,
    size: LogicalSize,
    pixels: (u32, u32),
    buffer: &MemoryRenderBuffer,
) -> Option<MemoryRenderBufferRenderElement<R>> {
    let side = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
    MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        on_output(sub_of(origin), view).to_f64(),
        buffer,
        None,
        Some(Rectangle::<f64, Logical>::from_size(
            (f64::from(pixels.0), f64::from(pixels.1)).into(),
        )),
        Some(Size::<i32, Logical>::from((side(size.w), side(size.h)))),
        Kind::Unspecified,
    )
    .ok()
}
