//! The two keys of a desktop entry that name an app's icon, and where the entry for an app id is.

use std::path::{Path, PathBuf};

/// What an app's `.desktop` file says about its icon and its window class.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopEntry {
    /// `Icon=`: a theme name or an absolute path.
    pub icon: Option<String>,
    /// `StartupWMClass=`: the `app_id` its windows carry, when that is not the file's name.
    pub window_class: Option<String>,
}

/// The `[Desktop Entry]` group of `text`. Other groups, comments and keys with a locale are
/// skipped; a key's last value wins.
///
/// ```
/// use casement_icons::parse_desktop_entry;
///
/// let got = parse_desktop_entry("[Desktop Entry]\nIcon=firefox\n[Desktop Action x]\nIcon=no\n");
/// assert_eq!(got.icon.as_deref(), Some("firefox"));
/// ```
pub fn parse(text: &str) -> DesktopEntry {
    text.lines()
        .map(str::trim)
        .fold(
            (false, DesktopEntry::default()),
            |(inside, entry), line| match (line.strip_prefix('['), line.split_once('=')) {
                (Some(group), _) => (group == "Desktop Entry]", entry),
                (None, Some((key, value))) if inside => (true, with_key(entry, key.trim(), value)),
                _ => (inside, entry),
            },
        )
        .1
}

/// `entry` with `key` set to `value` when it is one of the two this crate reads.
fn with_key(mut entry: DesktopEntry, key: &str, value: &str) -> DesktopEntry {
    let value = Some(value.trim().to_owned()).filter(|value| !value.is_empty());
    match key {
        "Icon" => entry.icon = value,
        "StartupWMClass" => entry.window_class = value,
        _ => {}
    }
    entry
}

/// The entry of the app whose windows carry `app_id`, searching `dirs` (the `applications`
/// folders, most important first): the file named `<app_id>.desktop`, else one whose
/// `StartupWMClass` is `app_id`, else one whose file name matches it in any case. Nothing when
/// no file does.
pub fn entry_for(dirs: &[PathBuf], app_id: &str) -> Option<DesktopEntry> {
    let named = format!("{app_id}.desktop");
    dirs.iter()
        .find_map(|dir| read(&dir.join(&named)))
        .or_else(|| {
            let all = all_entries(dirs);
            let by_class = all
                .iter()
                .find(|(_, entry)| entry.window_class.as_deref() == Some(app_id));
            let by_stem = all
                .iter()
                .find(|(stem, _)| stem.eq_ignore_ascii_case(app_id));
            by_class.or(by_stem).map(|(_, entry)| entry.clone())
        })
}

fn read(path: &Path) -> Option<DesktopEntry> {
    std::fs::read_to_string(path).ok().map(|text| parse(&text))
}

/// Every entry in `dirs` with its file stem: folders in the order given, files by name.
fn all_entries(dirs: &[PathBuf]) -> Vec<(String, DesktopEntry)> {
    dirs.iter()
        .flat_map(|dir| {
            let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .filter_map(|file| file.ok().map(|file| file.path()))
                .filter(|path| path.extension().is_some_and(|ext| ext == "desktop"))
                .collect();
            files.sort();
            files
        })
        .filter_map(|path| {
            let stem = path.file_stem()?.to_str()?.to_owned();
            Some((stem, read(&path)?))
        })
        .collect()
}
