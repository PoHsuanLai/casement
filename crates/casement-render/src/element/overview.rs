//! Mission Control's own drawing: a dimming layer over the desktop picture and a ring round the
//! window the pointer is on. The windows themselves are drawn by the scene as scaled windows;
//! the title under the hovered one is [`label_element`](super::label_element).

use super::{CasementElement, Rgba8, solid_rect};
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::scene::convert::{on_output, sub_of};
use crate::texture_book::TextureBook;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, PhysicalRect};
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::utils::CommitCounter;

/// The colours Mission Control draws with, premultiplied: the dimming over the picture and the
/// hovered window's ring. The composition root makes them from quire's tokens (the scrim and the
/// accent, in the person's scheme); this crate holds no colour of its own for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OverviewLook {
    /// The dimming over the picture.
    pub dim: Rgba8,
    /// The ring round the window the pointer is on.
    pub ring: Rgba8,
}

impl OverviewLook {
    /// A look from its two colours.
    ///
    /// ```
    /// use casement_render::{OverviewLook, Rgba8};
    ///
    /// let black = Rgba8 { r: 0, g: 0, b: 0, a: 56 };
    /// let blue = Rgba8 { r: 10, g: 96, b: 255, a: 255 };
    /// let look = OverviewLook::new(black, blue);
    /// assert_eq!((look.dim, look.ring), (black, blue));
    /// ```
    pub fn new(dim: Rgba8, ring: Rgba8) -> OverviewLook {
        OverviewLook { dim, ring }
    }
}

/// The ring's thickness, in logical pixels.
const THICKNESS: u32 = 3;

/// The dimming layer over all of `view`, one element.
pub fn dim_elements<R: Compositor>(
    view: &OutputView,
    book: &TextureBook,
    look: &OverviewLook,
) -> Vec<CasementElement<R>> {
    let area = PhysicalRect {
        x: 0,
        y: 0,
        w: view.mode.w,
        h: view.mode.h,
    };
    vec![CasementElement::Solid(solid_rect(
        book.cache().dim_id(),
        area,
        look.dim,
        CommitCounter::default(),
    ))]
}

/// A ring just outside `inner` (global logical pixels): four bars. Bars off this output draw
/// nothing.
pub fn ring_elements<R: Compositor>(
    inner: LogicalRect,
    view: &OutputView,
    look: &OverviewLook,
) -> Vec<CasementElement<R>> {
    let t = i32::try_from(THICKNESS).unwrap_or(3);
    let out = LogicalRect::new(
        LogicalPoint {
            x: inner.origin.x.saturating_sub(t),
            y: inner.origin.y.saturating_sub(t),
        },
        LogicalSize {
            w: inner.size.w.saturating_add(2 * THICKNESS),
            h: inner.size.h.saturating_add(2 * THICKNESS),
        },
    );
    let side = LogicalSize {
        w: THICKNESS,
        h: out.size.h.saturating_sub(2 * THICKNESS),
    };
    let across = LogicalSize {
        w: out.size.w,
        h: THICKNESS,
    };
    let bars = [
        LogicalRect::new(out.origin, across),
        LogicalRect::new(
            LogicalPoint {
                x: out.origin.x,
                y: out.origin.y.saturating_add_unsigned(out.size.h - THICKNESS),
            },
            across,
        ),
        LogicalRect::new(
            LogicalPoint {
                x: out.origin.x,
                y: out.origin.y.saturating_add(t),
            },
            side,
        ),
        LogicalRect::new(
            LogicalPoint {
                x: out.origin.x.saturating_add_unsigned(out.size.w - THICKNESS),
                y: out.origin.y.saturating_add(t),
            },
            side,
        ),
    ];
    bars.into_iter()
        .filter(|bar| view.area.intersection(bar).is_some())
        .map(|bar| {
            CasementElement::Solid(solid_rect(
                Id::new(),
                physical(bar, view),
                look.ring,
                CommitCounter::default(),
            ))
        })
        .collect()
}

/// `rect` on `view`, in its physical pixels: both corners rounded the way every element's are,
/// so the bars meet without a seam.
fn physical(rect: LogicalRect, view: &OutputView) -> PhysicalRect {
    let far = LogicalPoint {
        x: rect.origin.x.saturating_add_unsigned(rect.size.w),
        y: rect.origin.y.saturating_add_unsigned(rect.size.h),
    };
    let (from, to) = (
        on_output(sub_of(rect.origin), view),
        on_output(sub_of(far), view),
    );
    PhysicalRect {
        x: from.x,
        y: from.y,
        w: u32::try_from(to.x - from.x).unwrap_or(0),
        h: u32::try_from(to.y - from.y).unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::PhysicalSize;
    use ds_core::geometry::scale::Scale;

    fn view(scale: u32) -> OutputView {
        OutputView {
            area: LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 1280, h: 720 }),
            scale: Scale(scale),
            mode: PhysicalSize { w: 1280, h: 720 },
        }
    }

    #[test]
    fn a_rectangle_lands_on_whole_pixels_at_the_scale() {
        let rect = LogicalRect::new(LogicalPoint { x: 10, y: 20 }, LogicalSize { w: 100, h: 50 });
        // name, scale (120ths), want
        let cases = [
            (
                "1x",
                120,
                PhysicalRect {
                    x: 10,
                    y: 20,
                    w: 100,
                    h: 50,
                },
            ),
            (
                "2x",
                240,
                PhysicalRect {
                    x: 20,
                    y: 40,
                    w: 200,
                    h: 100,
                },
            ),
        ];
        for (name, scale, want) in cases {
            assert_eq!(physical(rect, &view(scale)), want, "{name}");
        }
    }
}
