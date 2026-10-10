//! Whether the cursor rides a hardware plane or is composited with the frame.

use crate::geometry::PhysicalSize;
use ds_core::geometry::scale::Scale;

/// What the output's cursor plane can do.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CursorPlaneCaps {
    /// The largest cursor the plane takes, when the output has a cursor plane.
    pub max_size: Option<PhysicalSize>,
}

impl CursorPlaneCaps {
    /// Caps.
    pub fn new(max_size: Option<PhysicalSize>) -> CursorPlaneCaps {
        CursorPlaneCaps { max_size }
    }
}

/// Where the cursor is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CursorPlaneChoice {
    /// On the hardware cursor plane.
    Hardware,
    /// Composited into the frame (no plane, or the image does not fit it).
    Composited,
}

/// The choice for a cursor image of `size` on an output at `scale`.
pub fn cursor_plane_for(
    caps: &CursorPlaneCaps,
    size: PhysicalSize,
    scale: Scale,
) -> CursorPlaneChoice {
    let _ = (caps, size, scale);
    todo!("P1-D3: cursor_plane_for")
}
