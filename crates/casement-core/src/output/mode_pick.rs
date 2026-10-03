//! Choosing a mode from what a display offers.

use super::{Mode, OutputFacts};
use crate::geometry::PhysicalSize;

/// Which mode is wanted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeWish {
    /// The display's preferred mode.
    Preferred,
    /// This mode exactly.
    Exact(Mode),
    /// The highest refresh rate at this resolution.
    HighestRefreshAt(PhysicalSize),
}

/// The mode `wish` picks from `facts.modes`, falling back to the preferred mode when the wish
/// cannot be met, and to the largest, fastest mode when none is marked preferred.
pub fn pick_mode(facts: &OutputFacts, wish: &ModeWish) -> Mode {
    let _ = (facts, wish);
    todo!("P0-F1-output: the wish, else preferred, else largest then fastest")
}
