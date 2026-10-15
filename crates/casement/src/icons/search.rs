//! Where the desktop entries are.

use std::path::PathBuf;

/// Whether app icons are looked for, and in which `applications` folders.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum IconSearch {
    /// No icons are looked for or drawn (the jail, the test kit).
    #[default]
    Off,
    /// Looked for through the desktop entries in these folders, most important first, and the
    /// installed icon themes.
    Dirs {
        /// The `applications` folders, most important first.
        applications: Vec<PathBuf>,
        /// The config folder the desktop's icon theme is named under (see
        /// `casement_icons::theme_file`); `hicolor` when none.
        config: Option<PathBuf>,
    },
}

impl IconSearch {
    /// The XDG folders: `$XDG_DATA_HOME` (else `$HOME/.local/share`) first, then each of
    /// `$XDG_DATA_DIRS` (else `/usr/local/share:/usr/share`), each with `applications` added;
    /// the config folder is `$XDG_CONFIG_HOME` (else `$HOME/.config`).
    ///
    /// ```
    /// use casement::icons::IconSearch;
    /// use std::path::PathBuf;
    ///
    /// let got = IconSearch::xdg(None, None, Some("/home/a".into()), Some("/x:/y".into()));
    /// let want = ["/home/a/.local/share/applications", "/x/applications", "/y/applications"];
    /// assert_eq!(
    ///     got,
    ///     IconSearch::Dirs {
    ///         applications: want.map(PathBuf::from).to_vec(),
    ///         config: Some(PathBuf::from("/home/a/.config")),
    ///     }
    /// );
    /// ```
    pub fn xdg(
        config_home: Option<PathBuf>,
        data_home: Option<PathBuf>,
        home: Option<PathBuf>,
        data_dirs: Option<String>,
    ) -> IconSearch {
        let config = config_home
            .filter(|dir| dir.is_absolute())
            .or_else(|| home.as_ref().map(|home| home.join(".config")));
        let first = data_home
            .filter(|dir| dir.is_absolute())
            .or_else(|| home.map(|home| home.join(".local/share")));
        let rest = data_dirs
            .filter(|dirs| !dirs.is_empty())
            .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned());
        let applications = first
            .into_iter()
            .chain(
                rest.split(':')
                    .filter(|dir| !dir.is_empty())
                    .map(PathBuf::from),
            )
            .map(|dir| dir.join("applications"))
            .collect();
        IconSearch::Dirs {
            applications,
            config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folders_follow_the_xdg_defaults() {
        let dirs = |search: IconSearch| match search {
            IconSearch::Dirs { applications, .. } => applications,
            IconSearch::Off => Vec::new(),
        };
        // name, data home, home, data dirs, want
        let cases = [
            (
                "both unset",
                None,
                Some("/h"),
                None,
                vec![
                    "/h/.local/share/applications",
                    "/usr/local/share/applications",
                    "/usr/share/applications",
                ],
            ),
            (
                "a data home wins over the home folder",
                Some("/d"),
                Some("/h"),
                Some("/a"),
                vec!["/d/applications", "/a/applications"],
            ),
            (
                "a relative data home is ignored",
                Some("d"),
                Some("/h"),
                Some("/a::/b"),
                vec![
                    "/h/.local/share/applications",
                    "/a/applications",
                    "/b/applications",
                ],
            ),
            (
                "no home at all",
                None,
                None,
                Some("/a"),
                vec!["/a/applications"],
            ),
        ];
        for (name, data_home, home, data_dirs, want) in cases {
            let got = IconSearch::xdg(
                None,
                data_home.map(PathBuf::from),
                home.map(PathBuf::from),
                data_dirs.map(str::to_owned),
            );
            let want: Vec<PathBuf> = want.into_iter().map(PathBuf::from).collect();
            assert_eq!(dirs(got), want, "{name}");
        }
    }

    #[test]
    fn the_config_folder_follows_the_xdg_defaults() {
        let config = |search: IconSearch| match search {
            IconSearch::Dirs { config, .. } => config,
            IconSearch::Off => None,
        };
        // name, config home, home, want
        let cases = [
            ("set", Some("/c"), Some("/h"), Some("/c")),
            ("unset", None, Some("/h"), Some("/h/.config")),
            (
                "relative is ignored",
                Some("c"),
                Some("/h"),
                Some("/h/.config"),
            ),
            ("nothing", None, None, None),
        ];
        for (name, config_home, home, want) in cases {
            let got = IconSearch::xdg(
                config_home.map(PathBuf::from),
                None,
                home.map(PathBuf::from),
                None,
            );
            assert_eq!(config(got), want.map(PathBuf::from), "{name}");
        }
    }
}
