//! `keyboard.*`.

use crate::config::convert;
use casement_core::settings::CasementSettings;
use ds_settings::schema::Page;
use ds_settings::{Count, Ms};
use serde::{Deserialize, Serialize};

/// `keyboard.*`: the keymap and the key repeat. An empty XKB name is libxkbcommon's own default
/// for it (and its `XKB_DEFAULT_*` environment): nothing here assumes a layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "casement/settings.toml", domain = "keyboard", page = Page::KeyboardAndShortcuts)]
pub struct KeyboardSection {
    /// `keyboard.xkb_rules`.
    #[settings(
        label = "XKB rules",
        help = "The XKB rules file; empty for the system's default.",
        section = "Layout",
        advanced
    )]
    pub xkb_rules: String,
    /// `keyboard.xkb_model`.
    #[settings(
        label = "XKB model",
        help = "The keyboard model; empty for the system's default.",
        section = "Layout",
        advanced
    )]
    pub xkb_model: String,
    /// `keyboard.xkb_layout`.
    #[settings(
        label = "Layouts",
        help = "The layouts, comma-separated; empty for the system's default.",
        section = "Layout"
    )]
    pub xkb_layout: String,
    /// `keyboard.xkb_variant`.
    #[settings(
        label = "Variants",
        help = "The variants, comma-separated, one per layout; empty for none.",
        section = "Layout"
    )]
    pub xkb_variant: String,
    /// `keyboard.xkb_options`.
    #[settings(
        label = "XKB options",
        help = "The XKB options, comma-separated; empty for the system's default.",
        section = "Layout",
        advanced
    )]
    pub xkb_options: String,
    /// `keyboard.repeat_delay_ms`.
    #[settings(
        label = "Delay until repeat",
        help = "How long a key is held before it repeats.",
        section = "Repeat",
        range = "100..=2000",
        unit = "ms"
    )]
    pub repeat_delay_ms: Ms,
    /// `keyboard.repeat_rate`.
    #[settings(
        label = "Key repeat rate",
        help = "How many times a second a held key repeats.",
        section = "Repeat",
        range = "1..=60",
        unit = "/s"
    )]
    pub repeat_rate: Count,
}

impl Default for KeyboardSection {
    fn default() -> Self {
        convert::keyboard_doc(&CasementSettings::defaults().keyboard)
    }
}
