//! A server-side frame as elements: the titlebar with its lights in front of the window's
//! surfaces, the shadow behind them. Both are casement's own rasters. The titlebar (`frame/raster.rs`)
//! is cached in the [`TextureBook`] per window, size, scale and look; the shadow is eight tiles
//! (`frame/shadow.rs`) cached per scale, radius and layers, with no window size in the key.

mod raster;
mod shadow;
mod shadow_set;
mod slices;
mod title;

pub(crate) use shadow::Tiles;
pub(crate) use shadow_set::{ShadowKey, ShadowSet, key as shadow_key};
pub(crate) use slices::MARGIN as SHADOW_MARGIN;
pub use title::TitleDraw;

/// The tiles of `look`'s shadow at `scale`.
pub(crate) fn shadow_tiles(look: &FrameLook, scale: Scale) -> Tiles {
    shadow::tiles(&look.shadow, look.radius, scale)
}

use super::CasementElement;
use super::window::alpha;
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::scene::convert::{on_output, sub_of};
use crate::texture_book::TextureBook;
use casement_core::WindowId;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, buffer_size};
use casement_core::window::{FrameLook, FrameStyle};
use ds_core::geometry::scale::Scale;
use ds_core::vocab::Fraction;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::memory::{
    MemoryRenderBuffer, MemoryRenderBufferRenderElement,
};
use smithay::utils::{Logical, Rectangle, Size, Transform};

/// One window's frame as it is drawn this frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameDraw<'a> {
    /// The frame's look and metrics.
    pub style: &'a FrameStyle,
    /// The window's title raster, when there is one.
    pub title: Option<&'a TitleDraw>,
    /// Where the window's content is, global logical pixels.
    pub content: LogicalRect,
    /// The window's opacity, which the frame takes.
    pub opacity: Fraction,
}

/// The frame `style` makes round the window `window` whose content is at `content`, on `view`.
/// Nothing for the part that is off this output. The first list goes in front of the window's
/// surfaces (the title, then the titlebar and the lights), the second behind them (the shadow).
/// `title`, when there is one, is laid over the titlebar. Both are drawn at the window's `opacity`,
/// so a window that fades in, or out into the dock, takes its frame with it.
pub fn frame_elements<R: Compositor>(
    window: WindowId,
    draw: FrameDraw<'_>,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> (Vec<CasementElement<R>>, Vec<CasementElement<R>>) {
    let FrameDraw {
        style,
        title,
        content,
        opacity,
    } = draw;
    let faded = alpha(opacity);
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
                whole(pixels.w, pixels.h),
                faded,
                &buffer,
            )
        })
        .map(CasementElement::Memory)
        .into_iter()
        .collect();
    let front = title_element(window, (style, title), &bar, faded, view, book, renderer)
        .into_iter()
        .chain(front)
        .collect();
    let back = shadow_elements(window, (style, outline), faded, (view, book), renderer);
    (front, back)
}

/// The title laid over the titlebar `bar`: the same size and place as the titlebar's raster, so
/// the pixels line up one to one. Nothing for no title, a title that is not that size, or a bar
/// off this output.
fn title_element<R: Compositor>(
    window: WindowId,
    (style, title): (&FrameStyle, Option<&TitleDraw>),
    bar: &LogicalRect,
    faded: f32,
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
        whole(pixels.w, pixels.h),
        faded,
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

/// The whole of a buffer `w` by `h` pixels.
fn whole(w: u32, h: u32) -> Rectangle<f64, Logical> {
    Rectangle::from_size((f64::from(w), f64::from(h)).into())
}

/// The window's shadow: its tiles drawn round `outline` (relative to the outline's corner, as
/// `slices` places them). The same eight tiles serve every size of window, so a window being
/// resized changes only the elements' places and sizes.
fn shadow_elements<R: Compositor>(
    window: WindowId,
    (style, outline): (&FrameStyle, LogicalRect),
    faded: f32,
    (view, book): (&OutputView, &TextureBook),
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let margin = i32::try_from(slices::MARGIN).unwrap_or(0);
    let reach = LogicalRect::new(
        LogicalPoint {
            x: outline.origin.x.saturating_sub(margin),
            y: outline.origin.y.saturating_sub(margin),
        },
        LogicalSize {
            w: outline.size.w + 2 * slices::MARGIN,
            h: outline.size.h + 2 * slices::MARGIN,
        },
    );
    if view.area.intersection(&reach).is_none() {
        return Vec::new();
    }
    let set = book
        .cache()
        .shadow(window, shadow_key(&style.look, view.scale), || {
            shadow_tiles(&style.look, view.scale)
        });
    slices::slices(outline.size, set.inner())
        .into_iter()
        .filter_map(|slice| {
            let dest = LogicalRect::new(
                LogicalPoint {
                    x: outline.origin.x.saturating_add(slice.dest.origin.x),
                    y: outline.origin.y.saturating_add(slice.dest.origin.y),
                },
                slice.dest.size,
            );
            view.area.intersection(&dest)?;
            let (buffer, raster) = set.of(slice.piece);
            let tile = slices::tile_size(slice.piece, set.inner());
            let per = |pixels: u32, logical: u32| f64::from(pixels) / f64::from(logical.max(1));
            let (px, py) = (per(raster.w, tile.w), per(raster.h, tile.h));
            let src = Rectangle::<f64, Logical>::new(
                (
                    f64::from(slice.src.origin.x) * px,
                    f64::from(slice.src.origin.y) * py,
                )
                    .into(),
                (
                    f64::from(slice.src.size.w) * px,
                    f64::from(slice.src.size.h) * py,
                )
                    .into(),
            );
            element(renderer, view, dest.origin, dest.size, src, faded, buffer)
                .map(CasementElement::Memory)
        })
        .collect()
}

/// `buffer` drawn with its top-left at the global logical `origin`, stretched to `size`
/// logical pixels; `src` is the part of the buffer shown, in its pixels.
fn element<R: Compositor>(
    renderer: &mut R,
    view: &OutputView,
    origin: LogicalPoint,
    size: LogicalSize,
    src: Rectangle<f64, Logical>,
    alpha: f32,
    buffer: &MemoryRenderBuffer,
) -> Option<MemoryRenderBufferRenderElement<R>> {
    let side = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
    MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        on_output(sub_of(origin), view).to_f64(),
        buffer,
        Some(alpha),
        Some(src),
        Some(Size::<i32, Logical>::from((side(size.w), side(size.h)))),
        Kind::Unspecified,
    )
    .ok()
}
