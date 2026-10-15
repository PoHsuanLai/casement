//! How many cascade steps a new window has walked before the walk starts over at the centre.
//!
//! This is the one piece of `RustScene`'s arithmetic repeated here: the CSS form (`mod()` and
//! `round()` with a variable step) did not lay out in blitz, so the cycle is counted in Rust and
//! `scene.css` places the window (centre, size and the walk itself). The cascade's parity check
//! therefore covers the size, the centre and the plumbing, not the cycle length.

use casement_core::geometry::LogicalSize;
use casement_core::window::MaxSize;
use casement_layout::{OutputSpec, WindowSpec};

/// The number of whole steps window `nth` (counting new windows of its Space from 0) is moved
/// from the centre, for `step` logical pixels on both axes.
pub(super) fn turn(
    output: &OutputSpec,
    window: &WindowSpec,
    asked: LogicalSize,
    nth: i64,
    step: i32,
) -> i64 {
    let area = output.usable;
    let taken = i64::from(window.reserve_top.min(area.size.h));
    let (uw, uh) = (i64::from(area.size.w), i64::from(area.size.h) - taken);
    let (w, h) = (
        held(asked.w, window.limits.min.w, bound(window, true), uw),
        held(asked.h, window.limits.min.h, bound(window, false), uh),
    );
    // Room left after centring, in whole pixels: the far side gets the odd pixel.
    let (room_x, room_y) = (uw - w - (uw - w) / 2, uh - h - (uh - h) / 2);
    let step = i64::from(step.max(1));
    let fit = (room_x / step).min(room_y / step);
    nth % (fit + 1)
}

fn bound(window: &WindowSpec, wide: bool) -> Option<u32> {
    match window.limits.max {
        MaxSize::Unbounded => None,
        MaxSize::Bounded(max) => Some(if wide { max.w } else { max.h }),
    }
}

/// `want` held to the maximum, raised to the minimum and one pixel, then held to `room`.
fn held(want: u32, min: u32, max: Option<u32>, room: i64) -> i64 {
    let capped = max.map_or(want, |max| want.min(max));
    i64::from(capped).max(i64::from(min)).max(1).min(room)
}
