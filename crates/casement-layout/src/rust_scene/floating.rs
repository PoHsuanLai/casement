//! A floating window's target: kept where it is when it fits; otherwise its size held to its
//! limits, then to the usable area (an unmet minimum is reported), and its position pushed
//! inside.

use crate::{LayoutReport, WindowTarget};
use casement_core::geometry::LogicalRect;
use casement_core::window::SizeLimits;
use casement_core::{OutputKey, WindowId};

/// Where a floating window at `rect` on `output` goes inside `usable`.
pub(crate) fn floating_target(
    window: WindowId,
    output: &OutputKey,
    rect: LogicalRect,
    limits: SizeLimits,
    usable: LogicalRect,
) -> (WindowTarget, Option<LayoutReport>) {
    let _ = (window, output, rect, limits, usable);
    todo!("P0-F1-layout: limits, then the area, then push inside")
}
