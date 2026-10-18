//! The motion book: every moving quantity's current leg.

use crate::space_slide::SlideLeg;
use crate::window_motion::WindowLegs;
use casement_core::time::Mono;
use casement_core::{OutputKey, WindowId};
use std::collections::BTreeMap;

/// Every moving quantity's leg. Holds ds-motion `Leg`s, whose state is float (springs are
/// closed-form arithmetic), so the book is `PartialEq` only; what leaves it is sampled and
/// quantised.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionBook {
    /// A window's rectangle and opacity legs.
    pub(crate) windows: BTreeMap<WindowId, WindowLegs>,
    /// An output's Space strip: the single leg of the x origin of its first Space.
    pub(crate) strips: BTreeMap<OutputKey, SlideLeg>,
}

impl MotionBook {
    /// Nothing moving.
    pub fn empty() -> MotionBook {
        MotionBook {
            windows: BTreeMap::new(),
            strips: BTreeMap::new(),
        }
    }

    /// Whether `window`'s legs are still moving at `now`; a window with no legs is at rest.
    pub fn window_is_moving(&self, window: WindowId, now: Mono) -> bool {
        self.windows
            .get(&window)
            .is_some_and(|legs| legs.is_moving(now))
    }

    /// Whether any leg is still moving at `now` (frames are wanted).
    pub fn is_moving(&self, now: Mono) -> bool {
        self.windows.values().any(|legs| legs.is_moving(now))
            || self.strips.values().any(|leg| leg.is_moving(now))
    }
}
