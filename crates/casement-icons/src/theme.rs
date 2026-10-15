//! The icon theme: a name and a size to a file.

use std::path::PathBuf;

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

impl ThemeLookup for SystemTheme {
    fn find(&self, name: &str, edge: u32) -> Option<PathBuf> {
        freedesktop_icons::lookup(name)
            .with_size(u16::try_from(edge).unwrap_or(u16::MAX))
            .with_theme(self.theme.as_deref().unwrap_or("hicolor"))
            .find()
    }
}
