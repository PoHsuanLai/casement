//! The numbers layout reads that are not in the spec.

use casement_core::geometry::LogicalPoint;
use casement_core::settings::CasementSettings;
use ds_core::vocab::Fraction;

/// Layout's tunables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutKnobs {
    /// How far each new window cascades from the previous one.
    pub cascade_step: LogicalPoint,
    /// The margin around tiled windows (zero when `windows.tile_margins` is off).
    pub tile_margin: u32,
    /// The gap between windows spread out in Mission Control.
    pub spread_gap: u32,
    /// The smallest a spread window may shrink to, as a share of its size.
    pub spread_floor: Fraction,
}

impl LayoutKnobs {
    /// The knobs `settings` give.
    pub fn of(settings: &CasementSettings) -> LayoutKnobs {
        let _ = settings;
        todo!("P0-F1-layout: cascade step from the titlebar height, margins from tile_margins")
    }
}
