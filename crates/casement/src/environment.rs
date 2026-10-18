//! What the `casement` binary reads from the process environment to build a [`RunConfig`] from
//! the parsed command line: where state and icons live, how X11 apps are served, the service
//! manager's sockets and the session bus. `main` only parses arguments and calls [`run_config`].

use crate::RunConfig;
use crate::cli::{StartConfig, StartMode, XwaylandSource};
use crate::endpoint::Endpoint;
use crate::icons::IconSearch;
use crate::notify::Watchdog;
use crate::space_store::{StateDirs, StateHome};
use crate::spawn::Scopes;
use crate::xwayland::{self, XwaylandConfig, XwaylandMode};
use casement_backend::BackendChoice;
use std::path::PathBuf;

/// The environment values state placement reads.
struct StateEnv {
    xdg_state_home: Option<PathBuf>,
    home: Option<PathBuf>,
    xdg_runtime_dir: Option<PathBuf>,
}

impl StateEnv {
    fn read() -> StateEnv {
        StateEnv {
            xdg_state_home: std::env::var_os("XDG_STATE_HOME").map(PathBuf::from),
            home: std::env::var_os("HOME").map(PathBuf::from),
            xdg_runtime_dir: std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from),
        }
    }
}

/// Where state files go. An explicit `--state-home DIR` (`flag` is `At`) always wins, for any
/// backend. Otherwise the headless backend keeps none (its runs are throwaway), and the others
/// use `$XDG_STATE_HOME/casement`, else `$HOME/.local/state/casement`; a missing home keeps none.
/// The session lock's marker goes to `$XDG_RUNTIME_DIR/casement` on those, when there is one.
fn state_home(flag: StateHome, backend: BackendChoice, env: &StateEnv) -> StateHome {
    if matches!(flag, StateHome::At(_)) {
        return flag;
    }
    if backend == BackendChoice::Headless {
        return StateHome::Ephemeral;
    }
    let base = env
        .xdg_state_home
        .clone()
        .or_else(|| env.home.as_ref().map(|home| home.join(".local/state")));
    base.map_or(StateHome::Ephemeral, |base| {
        let dirs = StateDirs::at(base.join("casement"));
        StateHome::At(match &env.xdg_runtime_dir {
            Some(runtime) => dirs.with_runtime(runtime.join("casement")),
            None => dirs,
        })
    })
}

/// Where app icons are looked for: nowhere on the headless backend (its runs are throwaway), else
/// the XDG data folders. `--safe` looks for none.
fn icon_search(backend: BackendChoice, mode: StartMode) -> IconSearch {
    if backend == BackendChoice::Headless || mode == StartMode::Safe {
        return IconSearch::Off;
    }
    IconSearch::xdg(
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
        std::env::var("XDG_DATA_DIRS").ok(),
    )
}

/// How X11 apps are served: a flag wins; else the settings file; else the satellite when it is
/// installed (`PATH`) and off when it is not. The headless backend (throwaway runs) keeps what
/// the flags said, which is off without them.
fn x11_setup(start: &StartConfig) -> XwaylandConfig {
    if start.mode == StartMode::Safe {
        return XwaylandConfig::off();
    }
    if start.xwayland_source == XwaylandSource::Flag || start.backend == BackendChoice::Headless {
        return start.xwayland.clone();
    }
    let path = std::env::var("PATH").unwrap_or_default();
    xwayland::from_file(
        &crate::config::xwayland_of(&start.config_home),
        &path,
        std::path::Path::is_file,
    )
}

/// The run `start` asks for, with everything the environment decides filled in.
#[must_use]
pub fn run_config(mut start: StartConfig) -> RunConfig {
    start.state_home = state_home(start.state_home.clone(), start.backend, &StateEnv::read());
    start.icons = icon_search(start.backend, start.mode);
    start.xwayland = x11_setup(&start);
    if start.xwayland.mode == XwaylandMode::Off {
        tracing::info!("older X11 apps: off");
    } else {
        tracing::info!(
            satellite = %start.xwayland.satellite.0.display(),
            "older X11 apps: served through xwayland-satellite"
        );
    }
    // Debug1 lives on the jail's private bus, which the jail's scripts name here.
    let session_bus = std::env::var("DBUS_SESSION_BUS_ADDRESS").ok();
    let debug_bus = session_bus.clone().map_or(Endpoint::Unset, Endpoint::At);
    // Compositor1, which hands the shell its connection, lives on the session bus.
    let compositor_bus = session_bus.map_or(Endpoint::Unset, Endpoint::At);
    // systemd's service protocol: where to say "ready", and how often to say "alive".
    let notify = Endpoint::of(std::env::var("NOTIFY_SOCKET").ok().as_deref());
    let watchdog = Watchdog::from_env(
        std::env::var("WATCHDOG_USEC").ok().as_deref(),
        std::env::var("WATCHDOG_PID").ok().as_deref(),
        std::process::id(),
    );
    // A binding's programs get scopes of their own in the person's service manager,
    // which listens under the runtime folder; a headless run has none.
    let scopes = match start.backend {
        BackendChoice::Headless => Scopes::Off,
        _ => Scopes::under(std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)),
    };
    RunConfig::new(start)
        .with_scopes(scopes)
        .with_notify(notify)
        .with_watchdog(watchdog)
        .with_debug_bus(debug_bus)
        .with_compositor_bus(compositor_bus)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{self, Command};

    fn at(dir: &str) -> StateHome {
        StateHome::At(StateDirs::at(PathBuf::from(dir)))
    }

    fn env(xdg: Option<&str>, home: Option<&str>) -> StateEnv {
        StateEnv {
            xdg_state_home: xdg.map(PathBuf::from),
            home: home.map(PathBuf::from),
            xdg_runtime_dir: None,
        }
    }

    #[test]
    fn a_safe_start_uses_no_icons_and_no_x11_whatever_the_files_say() {
        let args = ["--backend", "tty", "--safe"].map(str::to_owned);
        let Ok(Command::Run(start)) = cli::parse_args(&args) else {
            panic!("the arguments parse to a run");
        };
        assert_eq!(start.mode, StartMode::Safe);
        assert_eq!(icon_search(start.backend, start.mode), IconSearch::Off);
        assert_eq!(x11_setup(&start).mode, XwaylandMode::Off);
    }

    #[test]
    fn the_lock_marker_goes_to_the_runtime_directory_when_there_is_one() {
        let with_runtime = StateEnv {
            xdg_runtime_dir: Some(PathBuf::from("/run/user/1000")),
            ..env(Some("/x"), None)
        };
        let want = StateHome::At(
            StateDirs::at(PathBuf::from("/x/casement"))
                .with_runtime(PathBuf::from("/run/user/1000/casement")),
        );
        assert_eq!(
            state_home(StateHome::Ephemeral, BackendChoice::Tty, &with_runtime),
            want
        );
        assert_eq!(
            state_home(StateHome::Ephemeral, BackendChoice::Headless, &with_runtime),
            StateHome::Ephemeral,
            "a headless run keeps none"
        );
    }

    #[test]
    fn state_placement_follows_the_flag_then_the_backend_then_the_environment() {
        let cases = [
            (
                "flag, headless",
                at("/f"),
                BackendChoice::Headless,
                env(Some("/x"), Some("/h")),
                at("/f"),
            ),
            (
                "flag, tty",
                at("/f"),
                BackendChoice::Tty,
                env(Some("/x"), Some("/h")),
                at("/f"),
            ),
            (
                "headless, no flag",
                StateHome::Ephemeral,
                BackendChoice::Headless,
                env(Some("/x"), Some("/h")),
                StateHome::Ephemeral,
            ),
            (
                "tty, xdg",
                StateHome::Ephemeral,
                BackendChoice::Tty,
                env(Some("/x"), Some("/h")),
                at("/x/casement"),
            ),
            (
                "winit, home only",
                StateHome::Ephemeral,
                BackendChoice::Winit,
                env(None, Some("/h")),
                at("/h/.local/state/casement"),
            ),
            (
                "tty, no home",
                StateHome::Ephemeral,
                BackendChoice::Tty,
                env(None, None),
                StateHome::Ephemeral,
            ),
        ];
        for (name, flag, backend, env, want) in cases {
            assert_eq!(state_home(flag, backend, &env), want, "{name}");
        }
    }
}
