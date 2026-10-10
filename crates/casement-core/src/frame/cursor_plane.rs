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

/// The choice for a cursor image of `size` (logical pixels) on an output at `scale`.
///
/// Policy (P1-D3, FINDINGS "p1-d3"): the cursor rides the hardware plane whenever the output
/// has one and the image, scaled to physical pixels (rounded up), fits the plane's largest
/// size; otherwise it is composited into the frame. The plane is never used for a cursor that
/// does not fit (no downscaling: a blurred pointer is worse than one frame of extra work).
///
/// ```no_run
/// use casement_core::frame::{CursorPlaneCaps, CursorPlaneChoice, cursor_plane_for};
/// use casement_core::geometry::PhysicalSize;
/// use ds_core::geometry::scale::Scale;
///
/// let caps = CursorPlaneCaps::new(Some(PhysicalSize { w: 64, h: 64 }));
/// let choice = cursor_plane_for(&caps, PhysicalSize { w: 24, h: 24 }, Scale::ONE);
/// assert_eq!(choice, CursorPlaneChoice::Hardware);
/// ```
pub fn cursor_plane_for(
    caps: &CursorPlaneCaps,
    size: PhysicalSize,
    scale: Scale,
) -> CursorPlaneChoice {
    let Some(max) = caps.max_size else {
        return CursorPlaneChoice::Composited;
    };
    let on_plane = scaled(size.w, scale) <= max.w && scaled(size.h, scale) <= max.h;
    match on_plane {
        true => CursorPlaneChoice::Hardware,
        false => CursorPlaneChoice::Composited,
    }
}

/// `edge` logical pixels at `scale`, rounded up: the edge of the buffer the plane would hold.
fn scaled(edge: u32, scale: Scale) -> u32 {
    let wide = u64::from(edge) * u64::from(scale.numerator());
    let den = u64::from(Scale::DENOMINATOR);
    u32::try_from(wide.div_ceil(den)).unwrap_or(u32::MAX)
}
