//! The `casement` binary: arguments, the environment and an exit code around [`casement::run`].

#[cfg(feature = "debug-inject")]
mod debug_peer_units;

use casement::cli::{self, Command};
use casement::notify::{NotifySocket, Watchdog};
use casement::space_store::{StateDirs, StateHome};
use casement::{CompositorBus, DebugBus, RunConfig};
use casement_backend::BackendChoice;
use std::path::PathBuf;
use std::process::ExitCode;

/// The environment values state placement reads.
struct StateEnv {
    xdg_state_home: Option<PathBuf>,
    home: Option<PathBuf>,
}

impl StateEnv {
    fn read() -> StateEnv {
        StateEnv {
            xdg_state_home: std::env::var_os("XDG_STATE_HOME").map(PathBuf::from),
            home: std::env::var_os("HOME").map(PathBuf::from),
        }
    }
}

/// Where state files go. An explicit `--state-home DIR` (`flag` is `At`) always wins, for any
/// backend. Otherwise the headless backend keeps none (its runs are throwaway), and the others
/// use `$XDG_STATE_HOME/casement`, else `$HOME/.local/state/casement`; a missing home keeps none.
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
        StateHome::At(StateDirs::at(base.join("casement")))
    })
}

fn main() -> ExitCode {
    let filter = std::env::var("CASEMENT_LOG").unwrap_or_else(|_| "info".to_owned());
    casement::log::init(&filter);
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse_args(&args) {
        Ok(Command::Help) => {
            print!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Ok(Command::WriteSchema(dir)) => match casement::config::write_schema(&dir) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("casement: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::Run(mut start)) => {
            start.state_home =
                state_home(start.state_home.clone(), start.backend, &StateEnv::read());
            // Debug1 lives on the jail's private bus, which the jail's scripts name here.
            let session_bus = std::env::var("DBUS_SESSION_BUS_ADDRESS").ok();
            let debug_bus = session_bus.clone().map_or(DebugBus::Unset, DebugBus::At);
            // Compositor1, which hands the shell its connection, lives on the session bus.
            let compositor_bus = session_bus.map_or(CompositorBus::Unset, CompositorBus::At);
            // systemd's service protocol: where to say "ready", and how often to say "alive".
            let notify = NotifySocket::of(std::env::var("NOTIFY_SOCKET").ok().as_deref());
            let watchdog = Watchdog::from_env(
                std::env::var("WATCHDOG_USEC").ok().as_deref(),
                std::env::var("WATCHDOG_PID").ok().as_deref(),
                std::process::id(),
            );
            let config = RunConfig::new(start)
                .with_notify(notify)
                .with_watchdog(watchdog)
                .with_debug_bus(debug_bus)
                .with_compositor_bus(compositor_bus);
            #[cfg(feature = "debug-inject")]
            let config = match debug_peer_units::from_env() {
                Ok(table) => config.with_debug_peer_units(table),
                Err(error) => {
                    eprintln!("casement: {error}");
                    return ExitCode::from(64);
                }
            };
            match casement::run(config) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("casement: {error}");
                    ExitCode::from(error.exit_code())
                }
            }
        }
        Err(error) => {
            eprintln!("casement: {error}\n{}", cli::USAGE);
            ExitCode::from(64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Without `debug-inject` the variable has no effect because nothing in this build names it:
    /// the module that reads it is compiled only under that feature.
    #[test]
    fn the_peer_units_variable_is_read_only_by_a_debug_inject_module() {
        let source = include_str!("main.rs");
        let gate = "#[cfg(feature = \"debug-inject\")]\nmod debug_peer_units;";
        assert!(source.contains(gate), "the module is gated");
        let name = concat!("CASEMENT_DEBUG", "_PEER_UNITS");
        let uses = source.matches(name).count();
        assert_eq!(uses, 1, "main.rs names the variable only in this test");
    }

    fn at(dir: &str) -> StateHome {
        StateHome::At(StateDirs::at(PathBuf::from(dir)))
    }

    fn env(xdg: Option<&str>, home: Option<&str>) -> StateEnv {
        StateEnv {
            xdg_state_home: xdg.map(PathBuf::from),
            home: home.map(PathBuf::from),
        }
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
