//! Between our integer geometry and smithay's: global logical and sub-pixel positions to an
//! output's physical pixels, and rectangles both ways.

use crate::scene::OutputView;
use casement_core::geometry::{LogicalPoint, PhysicalRect, SubPoint, SubPx};
use ds_core::geometry::scale::Scale;
use smithay::utils::{Physical, Point, Rectangle};

/// Sub-pixels per logical pixel (`SubPx`'s unit).
const SUB: i64 = 120;

/// smithay's rectangle for `rect` (sizes beyond `i32` are held to it).
pub(crate) fn physical(rect: PhysicalRect) -> Rectangle<i32, Physical> {
    let side = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
    Rectangle::new((rect.x, rect.y).into(), (side(rect.w), side(rect.h)).into())
}

/// Our rectangle for smithay's `rect` (negative sizes become empty).
pub(crate) fn ours(rect: Rectangle<i32, Physical>) -> PhysicalRect {
    let side = |n: i32| u32::try_from(n).unwrap_or(0);
    PhysicalRect {
        x: rect.loc.x,
        y: rect.loc.y,
        w: side(rect.size.w),
        h: side(rect.size.h),
    }
}

/// A logical point as sub-pixels.
pub(crate) fn sub_of(point: LogicalPoint) -> SubPoint {
    SubPoint {
        x: SubPx::of_logical(point.x),
        y: SubPx::of_logical(point.y),
    }
}

/// Where global `point` lands on `view`, in its physical pixels: relative to the output's
/// top-left, times the scale, rounded half away from zero (the rule of
/// `casement_core::geometry::to_physical`, so aligned elements land on the same pixels on
/// every renderer).
pub(crate) fn on_output(point: SubPoint, view: &OutputView) -> Point<i32, Physical> {
    let x = axis(point.x, view.area.origin.x, view.scale);
    let y = axis(point.y, view.area.origin.y, view.scale);
    Point::from((x, y))
}

fn axis(at: SubPx, origin: i32, scale: Scale) -> i32 {
    let relative = i64::from(at.0) - i64::from(origin) * SUB;
    rounded(relative * i64::from(scale.numerator()))
}

/// `value / (120 × 120)`, rounded half away from zero, held to `i32`.
fn rounded(value: i64) -> i32 {
    let denominator = SUB * i64::from(Scale::DENOMINATOR);
    let half = denominator / 2;
    let quotient = match value >= 0 {
        true => (value + half) / denominator,
        false => (value - half) / denominator,
    };
    i32::try_from(quotient).unwrap_or(if quotient < 0 { i32::MIN } else { i32::MAX })
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalRect, LogicalSize, PhysicalSize};

    fn view(x: i32, scale: u32) -> OutputView {
        OutputView {
            area: LogicalRect::new(LogicalPoint { x, y: 0 }, LogicalSize { w: 1280, h: 720 }),
            scale: Scale(scale),
            mode: PhysicalSize { w: 1920, h: 1080 },
        }
    }

    #[test]
    fn global_points_land_on_output_pixels() {
        // name, output x, scale, point (logical sub-pixels), physical
        type Case = (&'static str, i32, u32, (i32, i32), (i32, i32));
        const CASES: &[Case] = &[
            ("origin 1x", 0, 120, (0, 0), (0, 0)),
            ("whole 1.5x", 0, 180, (100 * 120, 10 * 120), (150, 15)),
            ("odd 1.5x rounds up", 0, 180, (101 * 120, 0), (152, 0)),
            ("second output", 1280, 120, (1300 * 120, 0), (20, 0)),
            ("left of the output", 1280, 240, (1270 * 120, 0), (-20, 0)),
            ("half sub-pixel 1x", 0, 120, (60, -60), (1, -1)),
            ("a sub-pixel below half", 0, 120, (59, 0), (0, 0)),
        ];
        for (name, x, scale, (px, py), want) in CASES {
            let point = SubPoint {
                x: SubPx(*px),
                y: SubPx(*py),
            };
            assert_eq!(
                on_output(point, &view(*x, *scale)),
                Point::from(*want),
                "{name}"
            );
        }
    }

    #[test]
    fn rectangles_round_trip() {
        let rect = PhysicalRect {
            x: -3,
            y: 9,
            w: 40,
            h: 1,
        };
        assert_eq!(ours(physical(rect)), rect);
    }
}
