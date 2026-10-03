//! Choosing a mode from what a display offers.

use super::{Mode, ModePreference, OutputFacts};
use crate::geometry::PhysicalSize;
use crate::time::Millihertz;
use std::cmp::Reverse;

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

/// What a display that offers no mode at all yields: nothing to light. The edge leaves such an
/// output off, and no scale or place is derived from it.
const NO_MODE: Mode = Mode {
    size: PhysicalSize { w: 0, h: 0 },
    refresh: Millihertz(0),
    preference: ModePreference::Other,
};

/// The mode `wish` picks from `facts.modes`, falling back to the preferred mode when the wish
/// cannot be met, and to the largest, fastest mode when none is marked preferred. An exact
/// wish matches on resolution and refresh rate and returns the mode as the display lists it.
/// Ties go to the mode listed first. A display with no modes yields a mode of size 0 x 0.
pub fn pick_mode(facts: &OutputFacts, wish: &ModeWish) -> Mode {
    let wished = match wish {
        ModeWish::Preferred => None,
        ModeWish::Exact(mode) => exact(&facts.modes, mode),
        ModeWish::HighestRefreshAt(size) => fastest_at(&facts.modes, *size),
    };
    wished
        .or_else(|| preferred(&facts.modes))
        .or_else(|| largest_fastest(&facts.modes))
        .unwrap_or(NO_MODE)
}

fn exact(modes: &[Mode], wanted: &Mode) -> Option<Mode> {
    modes
        .iter()
        .find(|mode| mode.size == wanted.size && mode.refresh == wanted.refresh)
        .copied()
}

fn fastest_at(modes: &[Mode], size: PhysicalSize) -> Option<Mode> {
    modes
        .iter()
        .filter(|mode| mode.size == size)
        .min_by_key(|mode| Reverse(mode.refresh))
        .copied()
}

fn preferred(modes: &[Mode]) -> Option<Mode> {
    modes
        .iter()
        .find(|mode| mode.preference == ModePreference::Preferred)
        .copied()
}

fn area(size: PhysicalSize) -> u64 {
    u64::from(size.w) * u64::from(size.h)
}

fn largest_fastest(modes: &[Mode]) -> Option<Mode> {
    modes
        .iter()
        .min_by_key(|mode| Reverse((area(mode.size), mode.refresh)))
        .copied()
}
