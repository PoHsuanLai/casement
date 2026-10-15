//! `gestures.*`.

use crate::config::convert;
use casement_core::settings::{CasementSettings, Enabled, SwipeFingers};
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// `gestures.*`: the trackpad's multi-finger swipes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "casement/settings.toml", domain = "gestures", page = Page::MouseAndGestures)]
pub struct GesturesSection {
    /// `gestures.spaces`.
    #[settings(
        label = "Swipe between full-screen apps",
        help = "Swipe left or right with this many fingers to move between Spaces.",
        section = "More gestures"
    )]
    pub spaces: SwipeFingers,
    /// `gestures.mission_control`.
    #[settings(
        label = "Mission Control",
        help = "Swipe up with this many fingers to open Mission Control, down to close it.",
        section = "More gestures"
    )]
    pub mission_control: SwipeFingers,
    /// `gestures.natural`.
    #[settings(
        label = "Natural direction",
        help = "The Spaces follow the fingers; off, they move against them.",
        section = "More gestures"
    )]
    pub natural: Enabled,
}

impl Default for GesturesSection {
    fn default() -> Self {
        convert::gestures_doc(&CasementSettings::defaults().gestures)
    }
}
