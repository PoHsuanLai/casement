//! Which processes own an X11 app's windows, for force quit. Satellite makes one Wayland window
//! per X11 window under the `WM_CLASS` as its app id and forwards nothing else, so the owner is
//! asked of the X server: each top-level window's class, and the process the server read from
//! the window's connection (`xres`). What is left here is the decision, which is pure: an
//! app's windows are those whose class names it, their processes are what gets signalled, and
//! satellite, this process, init and pid 0 never are. The X server sits behind [`XWindows`].

use casement_core::app::AppKey;
use rustix::process::RawPid;
use std::collections::BTreeSet;

/// A window's `WM_CLASS`: the instance name and the class name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WmClass {
    /// The instance (usually the program's name).
    pub instance: String,
    /// The class (usually the program's name, capitalised); satellite's app id.
    pub class: String,
}

impl WmClass {
    /// The two names in a `WM_CLASS` property value (`instance\0class\0`); none for a value
    /// without both.
    pub fn parse(value: &[u8]) -> Option<WmClass> {
        let mut names = value
            .split(|byte| *byte == 0)
            .map(|name| String::from_utf8_lossy(name).into_owned());
        let instance = names.next().filter(|name| !name.is_empty())?;
        let class = names.next().filter(|name| !name.is_empty())?;
        Some(WmClass { instance, class })
    }

    /// Whether either name is `app`, ignoring case.
    fn names(&self, app: &str) -> bool {
        self.instance.eq_ignore_ascii_case(app) || self.class.eq_ignore_ascii_case(app)
    }
}

/// A top-level X11 window as the X server reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XWindow {
    /// Its `WM_CLASS`, when it has a readable one.
    pub class: Option<WmClass>,
    /// The process behind its connection, when the server could name one.
    pub pid: Option<RawPid>,
}

/// Where the top-level X11 windows come from: the X server in a session (`xres::XServer`), a
/// table in a test.
pub trait XWindows {
    /// Every top-level window. A server that cannot be reached has none (and says why in the
    /// log), so force quit then signals only what the Wayland side knows.
    fn top_levels(&self) -> Vec<XWindow>;
}

/// The processes force quit never signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spared {
    /// The running xwayland-satellite: it carries every X11 program.
    pub satellite: Option<RawPid>,
    /// This process.
    pub own: RawPid,
}

impl Spared {
    /// Whether `pid` may be signalled: a real process (not 0, not init), not casement and not
    /// the satellite.
    fn allows(&self, pid: RawPid) -> bool {
        pid > 1 && pid != self.own && Some(pid) != self.satellite
    }
}

/// The text X11 windows of `app` are named by: the id a claimed key declared (satellite's app
/// id is the window's class), or the name of a proven app.
pub fn name_of(app: &AppKey) -> String {
    let wire = app.wire();
    wire.strip_prefix("claimed:").unwrap_or(&wire).to_owned()
}

/// The processes behind the windows of `windows` that `app` names, but for the spared ones.
pub fn pids_of_app(windows: &[XWindow], app: &str, spared: &Spared) -> BTreeSet<RawPid> {
    windows
        .iter()
        .filter(|window| window.class.as_ref().is_some_and(|class| class.names(app)))
        .filter_map(|window| window.pid)
        .filter(|pid| spared.allows(*pid))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWN: RawPid = 500;
    const SATELLITE: RawPid = 600;

    fn window(instance: &str, class: &str, pid: Option<RawPid>) -> XWindow {
        XWindow {
            class: Some(WmClass {
                instance: instance.to_owned(),
                class: class.to_owned(),
            }),
            pid,
        }
    }

    fn spared() -> Spared {
        Spared {
            satellite: Some(SATELLITE),
            own: OWN,
        }
    }

    #[test]
    fn a_class_value_has_an_instance_and_a_class() {
        let table: [(&[u8], Option<(&str, &str)>); 6] = [
            (b"xterm\0XTerm\0", Some(("xterm", "XTerm"))),
            (b"xterm\0XTerm", Some(("xterm", "XTerm"))),
            (b"gimp-2.10\0Gimp-2.10\0", Some(("gimp-2.10", "Gimp-2.10"))),
            (b"xterm\0", None),
            (b"\0XTerm\0", None),
            (b"", None),
        ];
        for (value, wanted) in table {
            let wanted = wanted.map(|(instance, class)| WmClass {
                instance: instance.to_owned(),
                class: class.to_owned(),
            });
            assert_eq!(WmClass::parse(value), wanted, "{value:?}");
        }
    }

    #[test]
    fn an_app_is_its_windows_by_class_or_instance_ignoring_case() {
        let windows = [
            window("xterm", "XTerm", Some(1001)),
            window("gimp", "Gimp", Some(1002)),
            window("xterm", "XTerm", Some(1003)),
        ];
        let table = [
            ("XTerm", vec![1001, 1003]),
            ("xterm", vec![1001, 1003]),
            ("GIMP", vec![1002]),
            ("firefox", vec![]),
            ("", vec![]),
        ];
        for (app, wanted) in table {
            let pids = pids_of_app(&windows, app, &spared());
            assert_eq!(pids.into_iter().collect::<Vec<_>>(), wanted, "{app}");
        }
    }

    #[test]
    fn two_windows_of_one_process_are_one_pid() {
        let windows = [
            window("a", "App", Some(1001)),
            window("b", "App", Some(1001)),
        ];
        assert_eq!(pids_of_app(&windows, "App", &spared()).len(), 1);
    }

    #[test]
    fn the_satellite_this_process_init_and_pid_zero_are_never_named() {
        let windows = [
            window("a", "App", Some(SATELLITE)),
            window("a", "App", Some(OWN)),
            window("a", "App", Some(1)),
            window("a", "App", Some(0)),
            window("a", "App", Some(-4)),
            window("a", "App", Some(1001)),
        ];
        let pids = pids_of_app(&windows, "App", &spared());
        assert_eq!(pids.into_iter().collect::<Vec<_>>(), vec![1001]);
    }

    #[test]
    fn with_no_satellite_known_only_the_fixed_pids_are_spared() {
        let spared = Spared {
            satellite: None,
            own: OWN,
        };
        let windows = [
            window("a", "App", Some(SATELLITE)),
            window("a", "App", Some(OWN)),
        ];
        let pids = pids_of_app(&windows, "App", &spared);
        assert_eq!(pids.into_iter().collect::<Vec<_>>(), vec![SATELLITE]);
    }

    #[test]
    fn a_window_without_a_class_or_a_pid_names_nothing() {
        let windows = [
            XWindow {
                class: None,
                pid: Some(1001),
            },
            window("a", "App", None),
        ];
        assert!(pids_of_app(&windows, "App", &spared()).is_empty());
    }
}
