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
