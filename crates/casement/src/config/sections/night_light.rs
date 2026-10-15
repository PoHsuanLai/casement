//! `night_light.*`.

use crate::config::convert;
use casement_core::settings::{CasementSettings, NightLightMode};
use ds_settings::Count;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// `night_light.*`: the screen warms on a schedule. A display that takes warmth over DDC is
/// warmed by sill instead; this is for the rest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "casement/settings.toml", domain = "night_light", page = Page::Appearance)]
pub struct NightLightSection {
    /// `night_light.mode`.
    #[settings(
        label = "Night light",
        help = "Warm the screen's colours: never, all day, or on a schedule.",
        section = "Night light"
    )]
    pub mode: NightLightMode,
    /// `night_light.from`.
    #[settings(
        label = "Starts at",
        help = "When warming starts, in minutes after midnight (sunset, from the Clock settings, \
                is written here when Night light follows the sun).",
        section = "Night light",
        range = "0..=1439",
        unit = "min"
    )]
    pub from: Count,
    /// `night_light.to`.
    #[settings(
        label = "Ends at",
        help = "When warming ends, in minutes after midnight.",
        section = "Night light",
        range = "0..=1439",
        unit = "min"
    )]
    pub to: Count,
    /// `night_light.kelvin`.
    #[settings(
        label = "Warmth",
        help = "The colour temperature at night; lower is warmer.",
        section = "Night light",
        range = "1000..=6500",
        unit = "K"
    )]
    pub kelvin: Count,
    /// `night_light.fade_minutes`.
    #[settings(
        label = "Fade",
        help = "How long the screen takes to warm up and to cool down.",
        section = "Night light",
        range = "0..=120",
        unit = "min"
    )]
    pub fade_minutes: Count,
}

impl Default for NightLightSection {
    fn default() -> Self {
        convert::night_light_doc(&CasementSettings::defaults().night_light)
    }
}
