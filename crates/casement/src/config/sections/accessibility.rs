//! `accessibility.*`.

use crate::config::convert;
use casement_core::settings::{CasementSettings, Enabled};
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// `accessibility.*`: how the screen is drawn for people who need it plainer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "casement/settings.toml", domain = "accessibility", page = Page::Appearance)]
pub struct AccessibilitySection {
    /// `accessibility.reduce_transparency`.
    #[settings(
        label = "Reduce transparency",
        help = "Make the blurred, see-through parts of the desktop mostly solid. Takes effect at \
                once.",
        section = "Accessibility"
    )]
    pub reduce_transparency: Enabled,
}

impl Default for AccessibilitySection {
    fn default() -> Self {
        convert::accessibility_doc(&CasementSettings::defaults().accessibility)
    }
}
