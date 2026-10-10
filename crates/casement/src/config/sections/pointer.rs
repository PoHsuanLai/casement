//! `pointer.*`.

use crate::config::convert;
use casement_core::settings::{AccelProfile, CasementSettings, Enabled};
use ds_settings::Fraction;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// `pointer.*`: how mice and trackballs behave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "casement/settings.toml", domain = "pointer", page = Page::MouseAndGestures)]
pub struct PointerSection {
    /// `pointer.accel`.
    #[settings(
        label = "Pointer acceleration",
        help = "Adaptive speeds the pointer up with the device's speed; flat moves it in proportion.",
        section = "Mouse"
    )]
    pub accel: AccelProfile,
    /// `pointer.natural_scroll`.
    #[settings(
        label = "Natural scrolling",
        help = "Content follows the fingers.",
        section = "Mouse"
    )]
    pub natural_scroll: Enabled,
    /// `pointer.tap`.
    #[settings(
        label = "Tap to click",
        help = "A light tap counts as a click.",
        section = "Mouse"
    )]
    pub tap: Enabled,
    /// `pointer.scroll_factor`.
    #[settings(
        label = "Scroll speed",
        help = "How far a scroll step goes (stored per-mille of the device's own step: 1000 is one step).",
        section = "Mouse",
        range = "100..=4000"
    )]
    pub scroll_factor: Fraction,
    /// `pointer.disable_while_typing`.
    #[settings(
        label = "Ignore while typing",
        help = "The touchpad is ignored while keys are pressed.",
        section = "Mouse"
    )]
    pub disable_while_typing: Enabled,
}

impl Default for PointerSection {
    fn default() -> Self {
        convert::mouse_doc(&CasementSettings::defaults().pointer.mouse)
    }
}
