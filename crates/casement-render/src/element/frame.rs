//! A server-side frame as elements: the titlebar with its lights in front of the window's
//! surfaces, the shadow behind them. Both are casement's own rasters. The titlebar (`frame/raster.rs`)
//! is cached in the [`TextureBook`] per window, size, scale and look; the shadow is eight tiles
//! (`frame/shadow.rs`) cached per scale, radius and layers, with no window size in the key; a window
//! too small for the tiles has one raster of its own size, cached by size.

mod raster;
mod shadow;
mod shadow_set;
mod shrink;
mod slices;
mod title;

pub(crate) use raster::Raster;
pub(crate) use shadow::Tiles;
pub(crate) use shadow_set::{ShadowKey, ShadowSet, WholeShadow, key as shadow_key};
pub(crate) use slices::MARGIN as SHADOW_MARGIN;
pub use title::TitleDraw;

use shrink::Shrink;

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
use casement_core::window::{FrameGeometry, FrameLook, FrameMetrics, FrameStyle};
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
    /// Where the window's content is drawn, global logical pixels.
    pub content: LogicalRect,
    /// The size the content really has. When `content` is smaller (Mission Control shows the
    /// window shrunk), the frame is drawn at this size and shrunk with it: titlebar, lights,
    /// title and shadow, as one picture. The same as `content.size` for a frame at its own size.
    pub full: LogicalSize,
    /// The window's opacity, which the frame takes.
    pub opacity: Fraction,
}

/// The frame `style` makes round the window `window` whose content is at `content`, on `view`.
/// Nothing for the part that is off this output. The first list goes in front of the window's
/// surfaces (the title, then the titlebar and the lights), the second behind them (the shadow).
/// `title`, when there is one, is laid over the titlebar. Both are drawn at the window's `opacity`,
/// so a window that fades in, or out into the dock, takes its frame with it. A `content` smaller
/// than `full` shrinks the whole frame by the same factor.
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
        full,
        opacity,
    } = draw;
    let faded = alpha(opacity);
    let shrink = Shrink::of(full, content.size);
    let geometry = style.around(LogicalRect::new(content.origin, full));
    // The frame as it is rastered (`bar`, `outline`, at its own size) and where it is drawn.
    let (bar, outline) = (geometry.titlebar(), geometry.outline());
    let seen = Seen {
        bar: placed_about(shrink, content.origin, bar),
        outline: placed_about(shrink, content.origin, outline),
    };
    let front: Vec<CasementElement<R>> = view
        .area
        .intersection(&seen.bar)
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
                seen.bar.origin,
                seen.bar.size,
                whole(pixels.w, pixels.h),
                faded,
                &buffer,
            )
        })
        .map(CasementElement::Memory)
        .into_iter()
        .collect();
    let front = title_element(
        window,
        (style, title),
        (&bar, &seen.bar),
        faded,
        view,
        book,
        renderer,
    )
    .into_iter()
    .chain(front)
    .collect();
    let drawn_outline = LogicalRect::new(seen.outline.origin, outline.size);
    let back = shadow_elements(
        window,
        (style, drawn_outline, shrink),
        faded,
        (view, book),
        renderer,
    );
    (front, back)
}

/// The outline (titlebar and content) of a frame with `metrics` round `content`, a window whose
/// own size is `full`, as [`frame_elements`] draws it: the frame shrunk with the window, about
/// the content's top-left corner. The same as the frame's own outline when `content` is `full`.
///
/// ```
/// use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
/// use casement_core::window::FrameMetrics;
/// use casement_render::shown_outline;
///
/// let metrics = FrameMetrics::new(28, 12, 8, 13, 4, 12);
/// let at = LogicalPoint { x: 100, y: 200 };
/// let full = LogicalSize { w: 800, h: 600 };
/// let half = LogicalSize { w: 400, h: 300 };
/// let outline = shown_outline(metrics, LogicalRect::new(at, half), full);
/// // The titlebar is 14 px at half size: the outline starts 14 px above the content.
/// assert_eq!(outline, LogicalRect::new(LogicalPoint { x: 100, y: 186 }, LogicalSize { w: 400, h: 314 }));
/// ```
pub fn shown_outline(
    metrics: FrameMetrics,
    content: LogicalRect,
    full: LogicalSize,
) -> LogicalRect {
    let shrink = Shrink::of(full, content.size);
    let outline = FrameGeometry::new(metrics, LogicalRect::new(content.origin, full)).outline();
    placed_about(shrink, content.origin, outline)
}

/// `length` (a radius, a titlebar's height) of a frame whose window of `full` size is drawn at
/// `drawn`: shrunk with it, to the nearest pixel.
///
/// ```
/// use casement_core::geometry::LogicalSize;
/// use casement_render::shown_length;
///
/// let full = LogicalSize { w: 800, h: 600 };
/// assert_eq!(shown_length(10, LogicalSize { w: 400, h: 300 }, full), 5);
/// assert_eq!(shown_length(10, full, full), 10);
/// ```
pub fn shown_length(length: u32, drawn: LogicalSize, full: LogicalSize) -> u32 {
    u32::try_from(Shrink::of(full, drawn).length(i64::from(length))).unwrap_or(0)
}

/// Where the frame's parts are drawn: the titlebar and the outline, shrunk about the content's
/// corner.
struct Seen {
    bar: LogicalRect,
    outline: LogicalRect,
}

/// `rect` (a part of the frame at its own size) drawn shrunk about `corner`, the content's
/// top-left: a part is placed by its offset from the corner.
fn placed_about(shrink: Shrink, corner: LogicalPoint, rect: LogicalRect) -> LogicalRect {
    let offset = |at: i32, from: i32| at.saturating_sub(from);
    let relative = LogicalRect::new(
        LogicalPoint {
            x: offset(rect.origin.x, corner.x),
            y: offset(rect.origin.y, corner.y),
        },
        rect.size,
    );
    shrink.about(corner, relative)
}

/// The title laid over the titlebar: made for `bars.0`, the titlebar at its own size, and drawn
/// into `bars.1`, where the titlebar is drawn (smaller in Mission Control), so the pixels line up
/// with the titlebar's raster. Nothing for no title, a title that is not that size, or a bar off
/// this output.
fn title_element<R: Compositor>(
    window: WindowId,
    (style, title): (&FrameStyle, Option<&TitleDraw>),
    (bar, drawn): (&LogicalRect, &LogicalRect),
    faded: f32,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Option<CasementElement<R>> {
    let title = title?;
    view.area.intersection(drawn)?;
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
        drawn.origin,
        drawn.size,
        whole(pixels.w, pixels.h),
        faded,
        &buffer,
    )
    .map(CasementElement::Memory)
}

/// A label (a text raster made like a title) laid over `at`, global logical pixels: the same
/// size as `at` at the output's native scale, drawn opaque. Nothing for a raster of another size
/// or a place off this output. Mission Control's title under the hovered window.
pub fn label_element<R: Compositor>(
    label: &TitleDraw,
    at: LogicalRect,
    view: &OutputView,
    renderer: &mut R,
) -> Option<CasementElement<R>> {
    view.area.intersection(&at)?;
    let pixels = buffer_size(at.size, view.scale);
    if (label.width, label.height) != (pixels.w, pixels.h) {
        return None;
    }
    let buffer = to_buffer(&raster::Raster {
        w: label.width,
        h: label.height,
        argb: label.argb()?,
    });
    element(
        renderer,
        view,
        at.origin,
        at.size,
        whole(pixels.w, pixels.h),
        1.0,
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
/// resized changes only the elements' places and sizes. `outline` has the frame's own size and
/// the corner it is drawn at; `shrink` draws every piece shrunk about that corner.
fn shadow_elements<R: Compositor>(
    window: WindowId,
    (style, outline, shrink): (&FrameStyle, LogicalRect, Shrink),
    faded: f32,
    (view, book): (&OutputView, &TextureBook),
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let margin = i32::try_from(slices::MARGIN).unwrap_or(0);
    let reach = shrink.about(
        outline.origin,
        LogicalRect::new(
            LogicalPoint {
                x: -margin,
                y: -margin,
            },
            LogicalSize {
                w: outline.size.w + 2 * slices::MARGIN,
                h: outline.size.h + 2 * slices::MARGIN,
            },
        ),
    );
    if view.area.intersection(&reach).is_none() {
        return Vec::new();
    }
    let look = &style.look;
    let key = shadow_key(look, view.scale);
    if shadow::wanted(outline.size, shadow::inner_reach(&look.shadow, look.radius)) {
        let set = book
            .cache()
            .whole_shadow(window, key, (outline.size.w, outline.size.h), || {
                shadow::full(&look.shadow, look.radius, outline.size, view.scale)
            });
        let raster = set.raster();
        return element(
            renderer,
            view,
            reach.origin,
            reach.size,
            whole(raster.w, raster.h),
            faded,
            set.buffer(),
        )
        .map(CasementElement::Memory)
        .into_iter()
        .collect();
    }
    let set = book
        .cache()
        .shadow(window, key, || shadow_tiles(look, view.scale));
    slices::slices(outline.size, set.inner())
        .into_iter()
        .filter_map(|slice| {
            let dest = shrink.about(outline.origin, slice.dest);
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

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::PhysicalSize;

    fn view(scale: u32) -> OutputView {
        OutputView {
            area: LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 1280, h: 720 }),
            scale: Scale(scale),
            mode: PhysicalSize { w: 1280, h: 720 },
        }
    }

    const METRICS: fn() -> FrameMetrics = || FrameMetrics::new(28, 12, 8, 13, 4, 12);

    /// `rect` as the element for it lands: its corner on the output's pixels and its size in them.
    fn physical(rect: LogicalRect, view: &OutputView) -> (i32, i32, u32, u32) {
        let corner = on_output(sub_of(rect.origin), view);
        let size = buffer_size(rect.size, view.scale);
        (corner.x, corner.y, size.w, size.h)
    }

    #[test]
    fn a_shrunk_frame_is_drawn_into_the_scaled_rectangles_at_1x_and_2x() {
        let at = LogicalPoint { x: 100, y: 200 };
        let full = LogicalSize { w: 800, h: 600 };
        let content = LogicalRect::new(at, LogicalSize { w: 400, h: 300 });
        let shrink = Shrink::of(full, content.size);
        let geometry = FrameGeometry::new(METRICS(), LogicalRect::new(at, full));
        let bar = placed_about(shrink, at, geometry.titlebar());
        let outline = shown_outline(METRICS(), content, full);
        // name, scale (120ths), the bar and the outline on the output's pixels
        let cases = [
            ("1x", 120, (100, 186, 400, 14), (100, 186, 400, 314)),
            ("2x", 240, (200, 372, 800, 28), (200, 372, 800, 628)),
        ];
        for (name, scale, want_bar, want_outline) in cases {
            let view = view(scale);
            assert_eq!(physical(bar, &view), want_bar, "{name} bar");
            assert_eq!(physical(outline, &view), want_outline, "{name} outline");
        }
    }

    #[test]
    fn a_frame_at_its_own_size_is_where_it_always_was() {
        let at = LogicalPoint { x: 100, y: 200 };
        let size = LogicalSize { w: 800, h: 600 };
        let content = LogicalRect::new(at, size);
        let want = FrameGeometry::new(METRICS(), content);
        assert_eq!(shown_outline(METRICS(), content, size), want.outline());
        assert_eq!(
            placed_about(Shrink::of(size, size), at, want.titlebar()),
            want.titlebar()
        );
    }

    #[test]
    fn the_shrunk_shadow_reaches_a_shrunk_margin_round_the_shrunk_outline() {
        let full = LogicalSize { w: 800, h: 600 };
        let outline = LogicalRect::new(
            LogicalPoint { x: 100, y: 186 },
            LogicalSize { w: 800, h: 628 },
        );
        let shrink = Shrink::of(full, LogicalSize { w: 400, h: 300 });
        let margin = i32::try_from(slices::MARGIN).unwrap_or(0);
        let reach = shrink.about(
            outline.origin,
            LogicalRect::new(
                LogicalPoint {
                    x: -margin,
                    y: -margin,
                },
                LogicalSize {
                    w: outline.size.w + 2 * slices::MARGIN,
                    h: outline.size.h + 2 * slices::MARGIN,
                },
            ),
        );
        // 80 px of margin is 40 shrunk, on every side of the (400 x 314) drawn outline.
        assert_eq!(reach.origin, LogicalPoint { x: 60, y: 146 });
        assert_eq!(reach.size, LogicalSize { w: 480, h: 394 });
    }
}
