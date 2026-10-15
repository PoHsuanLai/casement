//! `xwayland.*`.

use ds_core::word::Word;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// Whether older X11 apps run (`xwayland.mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum XwaylandUse {
    /// They run when `xwayland-satellite` is installed (found on `PATH`), and not otherwise.
    #[default]
    Auto,
    /// They run through `xwayland.satellite`, or through `xwayland-satellite` on `PATH`.
    Satellite,
    /// They do not run: no `DISPLAY`.
    Off,
}

/// `xwayland.*`: how older X11 apps are served. Unlike the other sections it has no part in the
/// model's `CasementSettings` (it is read once, at start, by `main`), so its `Default` is written
/// here and `crate::xwayland::resolve` is its conversion.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ds_settings::SettingsSchema,
)]
#[serde(default)]
#[settings(file = "casement/settings.toml", domain = "xwayland", page = Page::Apps)]
pub struct XwaylandSection {
    /// `xwayland.mode`.
    #[settings(
        label = "Run older X11 apps",
        help = "Automatic runs them when the xwayland-satellite program is installed; or turn \
                them on or off. Takes effect at the next login.",
        section = "Older apps"
    )]
    pub mode: XwaylandUse,
    /// `xwayland.satellite`.
    #[settings(
        label = "X11 helper program",
        help = "The xwayland-satellite program to use, by its full path; empty finds it among \
                the installed programs. Takes effect at the next login.",
        section = "Older apps",
        advanced
    )]
    pub satellite: String,
}
