//! Where a shadow piece's corner lands on the output, before it is rounded to a pixel. The
//! pieces of a shadow (`slices.rs`) share edges in logical pixels; smithay turns a corner into a
//! pixel by rounding it and the far edge by rounding the corner plus the size. Given the corner
//! already rounded (what `on_output` returns), the far edge rounds from the wrong place at a
//! scale like 1.5 and misses its neighbour's corner by a pixel: a gap, or two pieces blended
//! over each other. Given the exact corner, both edges are `round(logical * scale)`, the same
//! number for the two pieces that share it.

use crate::scene::OutputView;
use casement_core::geometry::LogicalPoint;
use ds_core::geometry::scale::Scale;
use smithay::utils::{Physical, Point};

/// Where global `point` is on `view`, in physical pixels and not rounded.
pub(super) fn exact(point: LogicalPoint, view: &OutputView) -> Point<f64, Physical> {
    let along = |at: i32, origin: i32| {
        let relative = i64::from(at) - i64::from(origin);
        (relative * i64::from(view.scale.numerator())) as f64 / f64::from(Scale::DENOMINATOR)
    };
    Point::from((
        along(point.x, view.area.origin.x),
        along(point.y, view.area.origin.y),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalRect, LogicalSize, PhysicalSize};

    /// The far edge of a piece smithay draws from `corner` over `logical` pixels at `scale`.
    fn far_edge(corner: f64, logical: i32, scale: f64) -> i32 {
        (corner + f64::from(logical) * scale).round() as i32
    }

    #[test]
    fn pieces_that_share_an_edge_share_it_in_pixels_at_every_scale() {
        for numerator in [120, 150, 180, 240, 300] {
            let view = OutputView {
                area: LogicalRect::new(
                    LogicalPoint { x: 0, y: 0 },
                    LogicalSize { w: 1920, h: 1080 },
                ),
                scale: Scale(numerator),
                mode: PhysicalSize { w: 3840, h: 2160 },
            };
            let scale = view.scale.as_f64();
            // A corner 139 across, a strip 1321 wide, the next corner: odd widths at 1.5x.
            for (x, width) in [(-80, 219), (139, 1321), (1460, 219), (-3, 5)] {
                let first = exact(LogicalPoint { x, y: 0 }, &view).x;
                let next = exact(LogicalPoint { x: x + width, y: 0 }, &view).x;
                assert_eq!(
                    far_edge(first, width, scale),
                    next.round() as i32,
                    "{numerator}/120 from {x} over {width}"
                );
            }
        }
    }
}
