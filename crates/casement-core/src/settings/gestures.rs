//! Trackpad gesture settings (`gestures.*`): the multi-finger swipes that switch Spaces and
//! open Mission Control (03 section 4.2; System Settings, Trackpad, More Gestures).

use super::Enabled;
use ds_core::word::Word;
use serde::{Deserialize, Serialize};

/// How many fingers a swipe takes to do something, or that it does nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum SwipeFingers {
    /// The swipe does nothing; clients get it.
    Off,
    /// Three fingers.
    Three,
    /// Four fingers.
    Four,
}

impl SwipeFingers {
    /// Whether a swipe of `fingers` fingers is this one.
    ///
    /// ```
    /// use casement_core::settings::SwipeFingers;
    ///
    /// assert!(SwipeFingers::Four.matches(4));
    /// assert!(!SwipeFingers::Four.matches(3));
    /// assert!(!SwipeFingers::Off.matches(0));
    /// ```
    pub fn matches(self, fingers: u8) -> bool {
        match self {
            SwipeFingers::Off => false,
            SwipeFingers::Three => fingers == 3,
            SwipeFingers::Four => fingers == 4,
        }
    }
}

/// `gestures.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct GestureSettings {
    /// `gestures.spaces`: the horizontal swipe between Spaces.
    pub spaces: SwipeFingers,
    /// `gestures.mission_control`: the vertical swipe that opens (up) and closes (down)
    /// Mission Control.
    pub mission_control: SwipeFingers,
    /// `gestures.natural`: the Spaces follow the fingers; off, they move against them.
    pub natural: Enabled,
}

impl GestureSettings {
    /// Settings with these fields.
    pub fn new(
        spaces: SwipeFingers,
        mission_control: SwipeFingers,
        natural: Enabled,
    ) -> GestureSettings {
        GestureSettings {
            spaces,
            mission_control,
            natural,
        }
    }

    /// The Mac's defaults: four fingers for both, and the Spaces follow the fingers.
    pub fn defaults() -> GestureSettings {
        GestureSettings::new(SwipeFingers::Four, SwipeFingers::Four, Enabled::On)
    }
}
