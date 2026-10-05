//! `outputs.*`.

use crate::config::convert;
use casement_core::settings::CasementSettings;
use ds_core::word::Word;
use ds_settings::Count;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// How a display's first scale is chosen (`outputs.first_sight_scale`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum FirstSight {
    /// Aim at the Mac's density and prefer a whole scale (`ScalePolicy::WholeLikeMac`).
    WholeLikeMac,
    /// Always `outputs.fixed_scale`.
    Fixed,
}

/// `outputs.*`: what a display gets the first time it is seen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "casement/settings.toml", domain = "outputs", page = Page::Appearance)]
pub struct OutputsSection {
    /// `outputs.first_sight_scale`.
    #[settings(
        label = "Scale of a new display",
        help = "Choose the scale from the display's size and resolution, like a Mac, or give \
                every new display the same one.",
        section = "Displays"
    )]
    pub first_sight_scale: FirstSight,
    /// `outputs.fixed_scale`.
    #[settings(
        label = "Fixed scale",
        help = "The scale every new display gets when it is set to Fixed.",
        section = "Displays",
        range = "100..=400",
        unit = "%"
    )]
    pub fixed_scale: Count,
}

impl Default for OutputsSection {
    fn default() -> Self {
        convert::outputs_doc(&CasementSettings::defaults().outputs)
    }
}
