//! The `casement` binary.

#[cfg(feature = "debug-inject")]
mod debug;

use casement::cli::{self, Command};
use casement::display::{self, attach_display};
use casement::{State, clients, core_loop};
use casement_backend::BackendKind;
use porter_core::PeerIdentity;
use smithay::reexports::calloop::EventLoop;
use smithay::reexports::wayland_server::Display;
use smithay::wayland::socket::ListeningSocketSource;
use std::process::ExitCode;

fn main() -> ExitCode {
    casement::log::init();
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
        Ok(Command::Run(config)) => run(config),
        Err(error) => {
            eprintln!("casement: {error}\n{}", cli::USAGE);
            ExitCode::from(64)
        }
    }
}

/// Runs the compositor until it is asked to stop. A binary built with `testing` or
/// `debug-inject` refuses every backend but headless and refuses to start outside the jail
/// (exit 5, §3.2).
fn run(config: cli::StartConfig) -> ExitCode {
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    if let Some(refusal) = refuse_outside_the_jail(&config) {
        eprintln!("casement: {refusal}");
        return ExitCode::from(JAIL_BREACH);
    }
    match serve(config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("casement: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The exit code of a jail breach (11 §6.1).
#[cfg(any(feature = "testing", feature = "debug-inject"))]
const JAIL_BREACH: u8 = 5;

/// Why a binary that can mint test input will not start, if it will not.
#[cfg(any(feature = "testing", feature = "debug-inject"))]
fn refuse_outside_the_jail(config: &cli::StartConfig) -> Option<String> {
    use casement::jail_check::{JailCheck, require_jail};
    if config.backend != casement_backend::BackendChoice::Headless {
        return Some("this build refuses every backend but headless".to_owned());
    }
    match require_jail(std::path::Path::new("/")) {
        JailCheck::Inside => None,
        JailCheck::Outside(breach) => {
            Some(format!("this build runs only inside the jail ({breach:?})"))
        }
    }
}

/// Why the compositor stopped with an error.
#[derive(Debug, thiserror::Error)]
enum RunError {
    /// It could not start.
    #[error("start: {0}")]
    Start(#[from] casement::StartError),
    /// The event loop failed.
    #[error("event loop: {0}")]
    Loop(String),
}

/// The calls `casementctl debug` makes, when Debug1 is served. The D-Bus object that feeds this
/// channel starts in `casement-ipc` (`serve_debug`, an interface ask: casement itself reaches
/// neither zbus nor tokio); until it does, nothing sends and Debug1 is not on the bus.
#[cfg(feature = "debug-inject")]
fn debug_calls()
-> Option<smithay::reexports::calloop::channel::Channel<casement_ipc::dbus::DebugCall>> {
    None
}

/// Without `debug-inject` there is no Debug1.
#[cfg(not(feature = "debug-inject"))]
fn debug_calls() -> Option<std::convert::Infallible> {
    None
}

/// Starts the compositor, listens for clients, and runs the event loop: a turn dispatches
/// what is ready, drains queued input into the model, reads the nested window if there is
/// one, answers the debug object, and flushes every client.
fn serve(config: cli::StartConfig) -> Result<(), RunError> {
    let loop_error = |error: &dyn std::fmt::Display| RunError::Loop(error.to_string());
    let mut event_loop: EventLoop<'static, State> =
        EventLoop::try_new().map_err(|error| loop_error(&error))?;
    let handle = event_loop.handle();
    let mut display: Display<State> = Display::new().map_err(|error| loop_error(&error))?;
    let mut state = State::new(config, &mut display, &handle)?;
    let display = attach_display(display, &handle)?;
    let socket = ListeningSocketSource::new_auto().map_err(|error| loop_error(&error))?;
    eprintln!(
        "casement: listening on {}",
        socket.socket_name().to_string_lossy()
    );
    handle
        .insert_source(socket, |stream, (), state: &mut State| {
            let role = casement_core::policy::ConnectionRole::Public;
            if let Err(error) = clients::insert_client(state, stream, role, PeerIdentity::Unproven)
            {
                tracing::warn!(%error, "a client could not be inserted");
            }
        })
        .map_err(|error| loop_error(&error))?;
    #[cfg(feature = "debug-inject")]
    let debug = match debug_calls() {
        Some(calls) => Some(debug::attach(&handle, calls).map_err(RunError::Loop)?),
        None => None,
    };
    #[cfg(not(feature = "debug-inject"))]
    let _ = debug_calls();
    core_loop::announce_outputs(&mut state);
    let stop = event_loop.get_signal();
    #[cfg(feature = "debug-inject")]
    let wake = debug.as_ref().map(|_| debug::WAITING_TURN);
    #[cfg(not(feature = "debug-inject"))]
    let wake = None;
    event_loop
        .run(wake, &mut state, |state| {
            core_loop::turn(state);
            #[cfg(feature = "debug-inject")]
            if let Some(debug) = &debug {
                debug::turn(debug, state);
            }
            if let BackendKind::Winit(window) = &mut state.backend
                && window.dispatch().is_break()
            {
                stop.stop();
            }
            display::flush(&display);
        })
        .map_err(|error| loop_error(&error))
}
