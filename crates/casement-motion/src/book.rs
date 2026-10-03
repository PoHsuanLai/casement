//! The motion book: every moving quantity's current leg.

use crate::space_slide::SlideLeg;
use crate::window_motion::{Fate, WindowLegs};
use casement_core::time::Mono;
use casement_core::{OutputKey, WindowId};
use std::collections::BTreeMap;

/// What moves.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MotionKey {
    /// A window's rectangle and opacity.
    Window(WindowId),
    /// An output's Space strip: the x origin of its first Space.
    SpaceStrip(OutputKey),
    /// An output's overview progress (no frame field reads it yet: the windows travel to their
    /// slots on their own legs).
    Overview(OutputKey),
}

/// One key's legs.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Motion {
    /// A window's legs.
    Window(WindowLegs),
    /// A strip's single leg.
    Slide(SlideLeg),
}

impl Motion {
    /// Whether anything about it still changes at `now`.
    fn is_moving(&self, now: Mono) -> bool {
        match self {
            Motion::Window(legs) => legs.is_moving(now),
            Motion::Slide(leg) => leg.is_moving(now),
        }
    }

    /// Whether it has finished leaving: nothing of it is drawn any more.
    pub(crate) fn is_gone(&self, now: Mono) -> bool {
        match self {
            Motion::Window(legs) => legs.fate == Fate::Leaving && !legs.is_moving(now),
            Motion::Slide(_) => false,
        }
    }
}

/// Every moving quantity's leg. Holds ds-motion `Leg`s, whose state is float (springs are
/// closed-form arithmetic), so the book is `PartialEq` only; what leaves it is sampled and
/// quantised.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionBook {
    pub(crate) legs: BTreeMap<MotionKey, Motion>,
}

impl MotionBook {
    /// Nothing moving.
    pub fn empty() -> MotionBook {
        MotionBook {
            legs: BTreeMap::new(),
        }
    }

    /// Whether any leg is still moving at `now` (frames are wanted).
    pub fn is_moving(&self, now: Mono) -> bool {
        self.legs.values().any(|motion| motion.is_moving(now))
    }
}
