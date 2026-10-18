//! `spaces.*`.

use crate::config::convert;
use casement_core::settings::{CasementSettings, Enabled};
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// `spaces.*`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "casement/settings.toml", domain = "spaces", page = Page::Spaces)]
pub struct SpacesSection {
    /// `spaces.per_display`.
    #[settings(
        label = "Displays have separate Spaces",
        help = "Each display has its own Spaces.",
        section = "Spaces"
    )]
    pub per_display: Enabled,
    /// `spaces.auto_rearrange`.
    #[settings(
        label = "Automatically rearrange Spaces",
        help = "Spaces reorder themselves by most recent use.",
        section = "Spaces"
    )]
    pub auto_rearrange: Enabled,
    /// `spaces.switch_on_activate`.
    #[settings(
        label = "Switch to a Space with open windows",
        help = "Activating an app shows a Space that has its windows.",
        section = "Spaces"
    )]
    pub switch_on_activate: Enabled,
}

impl Default for SpacesSection {
    fn default() -> Self {
        convert::spaces_doc(CasementSettings::defaults().spaces)
    }
}
