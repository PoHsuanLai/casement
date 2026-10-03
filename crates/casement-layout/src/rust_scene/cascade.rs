//! The Mac's placement of new windows: the first centred in the usable area, each later one a
//! cascade step down and right of the one before.

use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};

/// Where the `nth` new window (0 first) of `size` goes in `usable`, cascading by `step`.
pub(crate) fn cascade_rect(
    usable: LogicalRect,
    size: LogicalSize,
    nth: u32,
    step: LogicalPoint,
) -> LogicalRect {
    let _ = (usable, size, nth, step);
    todo!("P0-F1-layout: centre, then step per earlier new window")
}
