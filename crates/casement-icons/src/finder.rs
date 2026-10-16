//! The order an icon is looked for in.

use crate::{DesktopEntry, IconName, Pixels, RgbaBuffer, ThemeLookup, Want, decode, entry_for};
use std::path::{Path, PathBuf};

/// Looks icons up: in the toplevel's own pixels, its icon name, the app's desktop entry, then the
/// app id as an icon name. A place that has nothing, or a file that does not decode, passes the
/// question on to the next one.
///
/// Only a desktop entry may name an icon by a path. The names a toplevel sets and its app id are
/// names of a theme's icons, never paths ([`IconName`]).
#[derive(Debug, Clone)]
pub struct Finder<T> {
    /// The `applications` folders to search for desktop entries, most important first.
    pub applications: Vec<PathBuf>,
    /// The icon theme.
    pub theme: T,
}

impl<T: ThemeLookup> Finder<T> {
    /// The icon for `want` as `edge` by `edge` premultiplied pixels, or nothing: the caller
    /// draws its plain plate. Reads files; do not call it per frame, nor on the loop thread.
    pub fn find(&self, want: &Want, edge: u32) -> Option<Pixels> {
        own_pixels(&want.buffers, edge)
            .or_else(|| {
                want.name
                    .as_deref()
                    .and_then(|name| self.in_theme(name, edge))
            })
            .or_else(|| {
                let app_id = want.app_id.as_deref()?;
                self.by_entry(app_id, edge)
                    .or_else(|| self.in_theme(app_id, edge))
            })
    }

    /// The theme's icon called `name`, when `name` is a plain icon name.
    fn in_theme(&self, name: &str, edge: u32) -> Option<Pixels> {
        let name = IconName::parse(name)?;
        let path = self.theme.find(name.as_str(), edge)?;
        decode::load(&path, edge).ok()
    }

    /// The icon `app_id`'s desktop entry names: a path when the entry gives one, else a theme's
    /// icon name.
    fn by_entry(&self, app_id: &str, edge: u32) -> Option<Pixels> {
        let DesktopEntry { icon, .. } = entry_for(&self.applications, app_id)?;
        let icon = icon?;
        match Path::new(&icon).is_absolute() {
            true => decode::load(Path::new(&icon), edge).ok(),
            false => self.in_theme(&icon, edge),
        }
    }
}

/// The buffer closest to `edge` pixels (a larger one before a smaller one at the same distance,
/// since shrinking keeps detail), scaled to it.
fn own_pixels(buffers: &[RgbaBuffer], edge: u32) -> Option<Pixels> {
    let best = buffers
        .iter()
        .min_by_key(|buffer| (buffer.edge.abs_diff(edge), buffer.edge < edge))?;
    decode::scale_straight(best, edge).ok()
}
