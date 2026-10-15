//! The production layout. At P0: desktop floating windows and the Mac's centre-then-cascade
//! for new ones. P3-Z adds tile zones, fill, full screen, Split View, the spread, Exposé, the
//! Space row and Show Desktop, one file each.
//!
//! The Mission Control spread (P3-Z, 03 §1.8; SP8 and 21-FREEZE-AMENDMENTS A21): windows keep
//! their order and rough relative positions (each axis's centres mapped monotonically onto the
//! area) and shrink by one common factor from a closed-form start
//! (`s² Σwh + s·g·Σ(w+h) + n·g² = 0.55·A`, replaced by 1 when within 20% of it); a greedy
//! pairwise push, in canonical order (position, then id), moves each window by the full overlap
//! (overlaps under 1e-6 count as none) for at most 24 passes per trial; each failed trial scales
//! by 0.85, never below the floor; at the floor a deterministic shelf pack is tried, and if that
//! fails too the windows are clamped inside and `LayoutReport::SpreadOverflow` is reported. No
//! refinement trials. App Exposé orders windows by `StackIndex` then id, minimised thumbnails at
//! 16:9.

mod cascade;
mod desktop;
mod floating;
mod icon_rect;
mod overview;
mod split;
mod spread;
mod spread_fit;
mod tile_zone;

pub use tile_zone::zone_rect;

use crate::{Arrangement, LayoutKnobs, SceneLayout, SceneSpec};

/// casement's own layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RustScene;

impl SceneLayout for RustScene {
    /// For each output: each Space's origin (the shown one at the usable origin, the others
    /// beside it in row order, one usable width apart), each floating window kept where it is
    /// when it fits the usable area, else its size held to its limits and then to the area and
    /// its position pushed inside, each new window centred in the usable area and every later
    /// new window of the same Space one cascade step further (starting over at the centre
    /// when a step would leave the area); minimised windows get no target. A window whose size
    /// changed gets a `Resize`; a minimum that cannot be met is reported
    /// (`LayoutReport::MinUnmet`).
    ///
    /// A filled window takes the usable area and a full-screen one the whole output.
    ///
    /// A tiled window takes its zone's half or quarter of the usable area inside the tile margin
    /// (`tile_zone.rs`).
    ///
    /// A Split View Space's two windows take the left and right of the whole output with an
    /// 8 px divider between them (`split.rs`).
    ///
    /// Every output is arranged as a desktop whatever its mode, so the windows keep their
    /// desktop targets; an output in Mission Control also gets an overview slot for each window
    /// of its shown desktop Space, spread without overlap below the Spaces bar's strip
    /// (`spread.rs`; `LayoutReport::SpreadOverflow` when the floor scale cannot separate them).
    /// App Exposé and Show Desktop arrive with P3-Z, one file each.
    fn arrange(&self, spec: &SceneSpec, knobs: &LayoutKnobs) -> Arrangement {
        let mut arrangement = Arrangement::default();
        for output in &spec.outputs {
            desktop::arrange_output(output, knobs, &mut arrangement);
            overview::arrange_overview(output, knobs, &mut arrangement);
        }
        arrangement
    }
}
