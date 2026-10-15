//! The icon theme: a name and a size to a file.

use std::path::{Path, PathBuf};

/// Where an icon name is in the installed themes.
pub trait ThemeLookup {
    /// The file for `name` at about `edge` pixels, if the theme (or the ones it falls back to)
    /// has one.
    fn find(&self, name: &str, edge: u32) -> Option<PathBuf>;
}

/// The installed themes, through the XDG icon-theme lookup. The folders it searches are the
/// process's (`$XDG_DATA_DIRS` and the home folder's icons).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemTheme {
    /// The theme the desktop is set to; `hicolor`, the one every theme falls back to, when none.
    theme: Option<String>,
}

impl SystemTheme {
    /// The lookup in `theme`, or in `hicolor` for none.
    pub fn new(theme: Option<String>) -> SystemTheme {
        SystemTheme { theme }
    }
}

/// The file the desktop's icon theme is named in, under the config folder `config`: the GTK
/// settings file, which sill reads too, so both follow one choice.
pub fn theme_file(config: &Path) -> PathBuf {
    config.join("gtk-3.0").join("settings.ini")
}

/// The theme `settings` (the text of a GTK settings file) names with `gtk-icon-theme-name`;
/// nothing when it names none.
///
/// ```
/// use casement_icons::theme_named;
///
/// let text = "[Settings]\ngtk-icon-theme-name = Papirus\n";
/// assert_eq!(theme_named(text), Some("Papirus".to_owned()));
/// assert_eq!(theme_named("[Settings]\n"), None);
/// ```
pub fn theme_named(settings: &str) -> Option<String> {
    settings.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        let name = value.trim();
        (key.trim() == "gtk-icon-theme-name" && !name.is_empty()).then(|| name.to_owned())
    })
}

impl SystemTheme {
    /// The lookup in the theme the desktop is set to (read from the settings file under the
    /// config folder `config`), or in `hicolor` when the file is missing or names none.
    pub fn of_desktop(config: &Path) -> SystemTheme {
        let named = std::fs::read_to_string(theme_file(config))
            .ok()
            .and_then(|text| theme_named(&text));
        SystemTheme::new(named)
    }
}

impl ThemeLookup for SystemTheme {
    fn find(&self, name: &str, edge: u32) -> Option<PathBuf> {
        freedesktop_icons::lookup(name)
            .with_size(u16::try_from(edge).unwrap_or(u16::MAX))
            .with_theme(self.theme.as_deref().unwrap_or("hicolor"))
            .find()
    }
}
