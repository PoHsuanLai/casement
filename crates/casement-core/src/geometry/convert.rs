//! The one conversion from logical to physical pixels at a scale in 120ths.

use super::{LogicalRect, LogicalSize, PhysicalRect, PhysicalSize};
use ds_core::geometry::scale::Scale;

/// `rect` in device pixels at `scale`. Edges are scaled and rounded (halves away from zero),
/// then the size is the distance between them, so rectangles that touch in logical space touch
/// in device space with no gap or overlap.
pub fn to_physical(rect: LogicalRect, scale: Scale) -> PhysicalRect {
    let n = i64::from(scale.numerator());
    let left = i64::from(rect.origin.x);
    let top = i64::from(rect.origin.y);
    let x0 = scaled(left, n);
    let y0 = scaled(top, n);
    let x1 = scaled(left + i64::from(rect.size.w), n);
    let y1 = scaled(top + i64::from(rect.size.h), n);
    PhysicalRect {
        x: to_i32(x0),
        y: to_i32(y0),
        w: to_u32(x1 - x0),
        h: to_u32(y1 - y0),
    }
}

/// The buffer a surface of `logical` size needs at `scale`: each side `round(l × s)`.
pub fn buffer_size(logical: LogicalSize, scale: Scale) -> PhysicalSize {
    let n = i64::from(scale.numerator());
    PhysicalSize {
        w: to_u32(scaled(i64::from(logical.w), n)),
        h: to_u32(scaled(i64::from(logical.h), n)),
    }
}

/// `value × numerator / 120`, rounded half away from zero.
fn scaled(value: i64, numerator: i64) -> i64 {
    let product = value * numerator;
    let denominator = i64::from(Scale::DENOMINATOR);
    let half = denominator / 2;
    if product >= 0 {
        (product + half) / denominator
    } else {
        (product - half) / denominator
    }
}

fn to_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

fn to_u32(value: i64) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::LogicalPoint;

    /// A name, a logical size, a scale in 120ths, the buffer size.
    type BufferCase = (&'static str, (u32, u32), u32, (u32, u32));

    #[test]
    fn buffer_size_rounds_each_side() {
        const CASES: &[BufferCase] = &[
            ("1x", (1001, 701), 120, (1001, 701)),
            ("1.25x", (1001, 701), 150, (1251, 876)),
            ("1.5x", (1001, 701), 180, (1502, 1052)),
            ("2x", (1001, 701), 240, (2002, 1402)),
        ];
        for (name, (w, h), scale, (pw, ph)) in CASES {
            let got = buffer_size(LogicalSize { w: *w, h: *h }, Scale(*scale));
            assert_eq!(got, PhysicalSize { w: *pw, h: *ph }, "{name}");
        }
    }

    #[test]
    fn touching_rects_stay_touching_at_a_fractional_scale() {
        let left = LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 3, h: 3 });
        let right = LogicalRect::new(LogicalPoint { x: 3, y: 0 }, LogicalSize { w: 3, h: 3 });
        let a = to_physical(left, Scale(150));
        let b = to_physical(right, Scale(150));
        assert_eq!(
            a,
            PhysicalRect {
                x: 0,
                y: 0,
                w: 4,
                h: 4
            }
        );
        assert_eq!(i64::from(a.x) + i64::from(a.w), i64::from(b.x));
        assert_eq!(
            b,
            PhysicalRect {
                x: 4,
                y: 0,
                w: 4,
                h: 4
            }
        );
    }

    #[test]
    fn negative_origins_round_away_from_zero() {
        let r = LogicalRect::new(LogicalPoint { x: -3, y: -1 }, LogicalSize { w: 2, h: 2 });
        assert_eq!(
            to_physical(r, Scale(180)),
            PhysicalRect {
                x: -5,
                y: -2,
                w: 3,
                h: 4
            }
        );
    }
}
