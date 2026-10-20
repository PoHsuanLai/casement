//! The two keys of a desktop entry that name an app's icon, and where the entry for an app id is.
//!
//! Reading and matching are `ds_helpers`' (the same walk and the same five rules the dock uses),
//! so Mission Control and the dock cannot pick different icons for one window.

use ds_helpers::{DESKTOP_ENTRY_CAP, DesktopFile, entry_for_window, load_entries};
use std::path::PathBuf;

/// What an app's `.desktop` file says about its icon and its window class.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopEntry {
    /// `Icon=`: a theme name or an absolute path.
    pub icon: Option<String>,
    /// `StartupWMClass=`: the `app_id` its windows carry, when that is not the file's name.
    pub window_class: Option<String>,
}

/// The `[Desktop Entry]` group of `text`. Other groups, comments and keys with a locale are
/// skipped; a key's last value wins; a text over the desktop-entry cap reads as empty.
///
/// ```
/// use casement_icons::parse_desktop_entry;
///
/// let got = parse_desktop_entry("[Desktop Entry]\nIcon=firefox\n[Desktop Action x]\nIcon=no\n");
/// assert_eq!(got.icon.as_deref(), Some("firefox"));
/// ```
pub fn parse(text: &str) -> DesktopEntry {
    let Ok(file) = DesktopFile::parse(text, DESKTOP_ENTRY_CAP) else {
        return DesktopEntry::default();
    };
    let value = |key: &str| {
        let value = file.entry()?.string(key)?.trim().to_owned();
        (!value.is_empty()).then_some(value)
    };
    DesktopEntry {
        icon: value("Icon"),
        window_class: value("StartupWMClass"),
    }
}

/// The entry of the app whose windows carry `app_id`, searching `dirs` (the `applications`
/// folders, subfolders too, most important first) by `ds_helpers::match_app`'s five rules: the
/// file's id, the id in any case, `StartupWMClass`, the id's last labels, the Flatpak id. Nothing
/// when no file does.
pub fn entry_for(dirs: &[PathBuf], app_id: &str) -> Option<DesktopEntry> {
    let entries = load_entries(dirs);
    let found = entry_for_window(&entries, app_id, None)?;
    Some(DesktopEntry {
        icon: found.icon(),
        window_class: found.keys.startup_wm_class.clone(),
    })
}
