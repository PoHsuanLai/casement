//! `touchpad.*`.

use crate::config::convert;
use casement_core::settings::{AccelProfile, CasementSettings, Enabled};
use ds_settings::Fraction;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// `touchpad.*`: how touchpads behave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "casement/settings.toml", domain = "touchpad", page = Page::MouseAndGestures)]
pub struct TouchpadSection {
    /// `touchpad.accel`.
    #[settings(
        label = "Pointer acceleration",
        help = "Adaptive speeds the pointer up with the device's speed; flat moves it in proportion.",
        section = "Trackpad"
    )]
    pub accel: AccelProfile,
    /// `touchpad.natural_scroll`.
    #[settings(
        label = "Natural scrolling",
        help = "Content follows the fingers.",
        section = "Trackpad"
    )]
    pub natural_scroll: Enabled,
    /// `touchpad.tap`.
    #[settings(
        label = "Tap to click",
        help = "A light tap counts as a click.",
        section = "Trackpad"
    )]
    pub tap: Enabled,
    /// `touchpad.scroll_factor`.
    #[settings(
        label = "Scroll speed",
        help = "How far a scroll step goes (stored per-mille of the device's own step: 1000 is one step).",
        section = "Trackpad",
        range = "100..=4000"
    )]
    pub scroll_factor: Fraction,
    /// `touchpad.disable_while_typing`.
    #[settings(
        label = "Ignore while typing",
        help = "The touchpad is ignored while keys are pressed.",
        section = "Trackpad"
    )]
    pub disable_while_typing: Enabled,
}

impl Default for TouchpadSection {
    fn default() -> Self {
        convert::touchpad_doc(&CasementSettings::defaults().pointer.touchpad)
    }
}
