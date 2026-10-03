//! Taffy's float layout back to whole logical pixels: the one rounding step on layout's side.

use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use taffy::Layout;

/// A node's layout as a logical rectangle offset by `origin`: the edges are rounded (halves
/// away from zero) and the size is the distance between them, so neighbours never overlap.
pub(crate) fn rect_of(layout: &Layout, origin: LogicalPoint) -> LogicalRect {
    let left = layout.location.x;
    let top = layout.location.y;
    let x0 = whole(left);
    let y0 = whole(top);
    let x1 = whole(left + layout.size.width);
    let y1 = whole(top + layout.size.height);
    LogicalRect::new(
        LogicalPoint {
            x: origin.x.saturating_add(x0),
            y: origin.y.saturating_add(y0),
        },
        LogicalSize {
            w: x1.saturating_sub(x0).max(0).unsigned_abs(),
            h: y1.saturating_sub(y0).max(0).unsigned_abs(),
        },
    )
}

/// Logical pixels as Taffy's float (exact below 2^24, far beyond any output).
pub(super) fn to_float(value: u32) -> f32 {
    value as f32
}

/// `value` rounded to a whole logical pixel (saturating; NaN is 0).
fn whole(value: f32) -> i32 {
    value.round() as i32
}
