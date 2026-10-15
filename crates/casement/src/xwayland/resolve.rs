//! Which X11 setup a run gets when the command line did not say: the `xwayland.*` keys of the
//! settings file, and for `auto` whether `xwayland-satellite` is installed. The environment is
//! read by `main`; the functions here take its values, so a table covers them.

use super::{SatellitePath, SatelliteRendering, XwaylandConfig, XwaylandMode};
use crate::config::{XwaylandSection, XwaylandUse};
use std::path::{Path, PathBuf};

/// The program's name, looked up in `PATH`.
pub const SATELLITE: &str = "xwayland-satellite";

/// The first `name` in the directories of `path` (a `PATH` value) for which `is_file` says yes;
/// an empty entry is skipped (it would mean the current directory).
///
/// ```
/// use casement::xwayland::on_path;
/// use std::path::Path;
///
/// let found = on_path("tool", "/a:/b", |file| file == Path::new("/b/tool"));
/// assert_eq!(found, Some("/b/tool".into()));
/// ```
pub fn on_path(name: &str, path: &str, is_file: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    path.split(':')
        .filter(|dir| !dir.is_empty())
        .map(|dir| Path::new(dir).join(name))
        .find(|file| is_file(file))
}

/// The satellite the file names (by its path, or by a name in `PATH`), else the one called
/// [`SATELLITE`] in `PATH`; none when it is not there.
fn found(named: &str, path: &str, is_file: &impl Fn(&Path) -> bool) -> Option<PathBuf> {
    match named.trim() {
        "" => on_path(SATELLITE, path, is_file),
        by_path if by_path.contains('/') => {
            Some(PathBuf::from(by_path)).filter(|file| is_file(file))
        }
        by_name => on_path(by_name, path, is_file),
    }
}

/// What the `xwayland.*` keys ask for, given `path` (the `PATH` value) and `is_file`.
///
/// `auto` serves X11 clients through the satellite when it is installed and not at all
/// otherwise; `satellite` always serves them (a satellite that is missing is reported when the
/// first X11 client starts it); `off` never does.
pub fn from_file(
    file: &XwaylandSection,
    path: &str,
    is_file: impl Fn(&Path) -> bool,
) -> XwaylandConfig {
    let located = found(&file.satellite, path, &is_file);
    let serving = |program: PathBuf| XwaylandConfig {
        mode: XwaylandMode::Satellite,
        satellite: SatellitePath(program),
        rendering: SatelliteRendering::Automatic,
    };
    match (file.mode, located) {
        (XwaylandUse::Off, _) | (XwaylandUse::Auto, None) => XwaylandConfig::off(),
        (_, Some(program)) => serving(program),
        (XwaylandUse::Satellite, None) => serving(PathBuf::from(match file.satellite.trim() {
            "" => SATELLITE,
            named => named,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(mode: XwaylandUse, satellite: &str) -> XwaylandSection {
        XwaylandSection {
            mode,
            satellite: satellite.to_owned(),
        }
    }

    fn installed(files: &[&str]) -> impl Fn(&Path) -> bool {
        let files: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
        move |file| files.iter().any(|installed| installed == file)
    }

    #[test]
    fn a_name_is_looked_up_in_path_order() {
        let cases = [
            ("first wins", "/a:/b", &["/a/t", "/b/t"][..], Some("/a/t")),
            ("later entry", "/a:/b", &["/b/t"][..], Some("/b/t")),
            ("absent", "/a:/b", &["/c/t"][..], None),
            (
                "empty entry skipped",
                ":/b",
                &["t", "/b/t"][..],
                Some("/b/t"),
            ),
            ("empty path", "", &["/t"][..], None),
        ];
        for (name, path, files, want) in cases {
            let got = on_path("t", path, installed(files));
            assert_eq!(got, want.map(PathBuf::from), "{name}");
        }
    }

    #[test]
    fn the_file_and_the_machine_pick_the_x11_setup() {
        let satellite = |path: &str| XwaylandConfig::satellite(path);
        let cases = [
            (
                "auto, installed",
                section(XwaylandUse::Auto, ""),
                &["/usr/bin/xwayland-satellite"][..],
                satellite("/usr/bin/xwayland-satellite"),
            ),
            (
                "auto, not installed",
                section(XwaylandUse::Auto, ""),
                &[][..],
                XwaylandConfig::off(),
            ),
            (
                "auto, own path present",
                section(XwaylandUse::Auto, "/opt/xs"),
                &["/opt/xs"][..],
                satellite("/opt/xs"),
            ),
            (
                "auto, own path missing",
                section(XwaylandUse::Auto, "/opt/xs"),
                &["/usr/bin/xwayland-satellite"][..],
                XwaylandConfig::off(),
            ),
            (
                "off, installed",
                section(XwaylandUse::Off, ""),
                &["/usr/bin/xwayland-satellite"][..],
                XwaylandConfig::off(),
            ),
            (
                "satellite, installed",
                section(XwaylandUse::Satellite, ""),
                &["/usr/bin/xwayland-satellite"][..],
                satellite("/usr/bin/xwayland-satellite"),
            ),
            (
                "satellite, not installed",
                section(XwaylandUse::Satellite, ""),
                &[][..],
                satellite("xwayland-satellite"),
            ),
            (
                "satellite, a name in path",
                section(XwaylandUse::Satellite, "xs"),
                &["/usr/bin/xs"][..],
                satellite("/usr/bin/xs"),
            ),
            (
                "satellite, own path missing",
                section(XwaylandUse::Satellite, "/opt/xs"),
                &[][..],
                satellite("/opt/xs"),
            ),
        ];
        for (name, file, files, want) in cases {
            let got = from_file(&file, "/usr/local/bin:/usr/bin", installed(files));
            assert_eq!(got, want, "{name}");
        }
    }
}
