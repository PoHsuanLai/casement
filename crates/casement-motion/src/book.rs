//! The motion book: every moving quantity's current leg.

use crate::space_slide::SlideLeg;
use crate::window_motion::WindowLegs;
use casement_core::time::Mono;
use casement_core::{OutputKey, WindowId};
use std::collections::BTreeMap;

/// What moves.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MotionKey {
    /// A window's rectangle and opacity.
    Window(WindowId),
    /// An output's Space strip.
    SpaceStrip(OutputKey),
    /// An output's overview progress.
    Overview(OutputKey),
}

/// One key's legs.
#[expect(
    dead_code,
    reason = "retarget builds the legs and sample reads them (P0-F1-motion)"
)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Motion {
    /// A window's legs.
    Window(WindowLegs),
    /// A strip's or an overview's single leg.
    Slide(SlideLeg),
}

/// Every moving quantity's leg. Holds ds-motion `Leg`s, whose state is float (springs are
/// closed-form arithmetic), so the book is `PartialEq` only; what leaves it is sampled and
/// quantised.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionBook {
    legs: BTreeMap<MotionKey, Motion>,
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
        let _ = (now, &self.legs);
        todo!("P0-F1-motion: any leg not at rest at now")
    }
}
