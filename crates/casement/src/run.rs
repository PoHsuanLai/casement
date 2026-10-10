//! Running the compositor: [`run`] starts casement from a [`RunConfig`] and drives its event
//! loop until it is asked to stop. The binary is only arguments, the environment and an exit
//! code around this; a test harness or a second binary embeds the compositor the same way.

#[cfg(feature = "debug-inject")]
use crate::debug;
use crate::display::{self, attach_display};
use crate::test_mode::{HookError, TestHooks};
use crate::{StartError, State, cli, clients, core_loop};
use casement_backend::BackendKind;
use casement_ipc::IpcError;
use casement_ipc::sleep::SleepBus;
use porter_core::PeerIdentity;
use smithay::reexports::calloop::{Error as LoopError, EventLoop};
use smithay::reexports::wayland_server::backend::InitError;
use smithay::reexports::wayland_server::{BindError, Display};
use smithay::wayland::socket::ListeningSocketSource;

/// The exit code of a jail breach (11 §6.1).
#[cfg(any(feature = "testing", feature = "debug-inject"))]
const JAIL_BREACH: u8 = 5;

/// Where Debug1 is served (feature `debug-inject`; ignored without it).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum DebugBus {
    /// No bus address was given.
    #[default]
    Unset,
    /// The private bus at this address (the jail's `$DBUS_SESSION_BUS_ADDRESS`).
    At(String),
}

/// What [`run`] needs: how casement starts, and where the debug bus is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RunConfig {
    /// How casement starts.
    pub start: cli::StartConfig,
    /// Where Debug1 is served.
    pub debug_bus: DebugBus,
    /// Which bus login1 is heard on (a tty seat hears the machine go to sleep there).
    pub sleep_bus: SleepBus,
}

impl RunConfig {
    /// A run that starts as `start` says, with no debug bus.
    pub fn new(start: cli::StartConfig) -> RunConfig {
        RunConfig {
            start,
            debug_bus: DebugBus::Unset,
            sleep_bus: SleepBus::System,
        }
    }

    /// The same run with Debug1 on `debug_bus`.
    #[must_use]
    pub fn with_debug_bus(self, debug_bus: DebugBus) -> RunConfig {
        RunConfig { debug_bus, ..self }
    }

    /// The same run hearing login1 on `sleep_bus`.
    #[must_use]
    pub fn with_sleep_bus(self, sleep_bus: SleepBus) -> RunConfig {
        RunConfig { sleep_bus, ..self }
    }
}

/// Why the compositor stopped with an error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RunError {
    /// It could not start.
    #[error("start: {0}")]
    Start(#[from] StartError),
    /// The event loop failed.
    #[error("event loop: {0}")]
    Loop(#[from] LoopError),
    /// The Wayland display could not be created.
    #[error("display: {0}")]
    Display(#[from] InitError),
    /// The client socket could not be bound.
    #[error("socket: {0}")]
    Socket(#[from] BindError),
    /// The vkms test mode's hooks could not be attached.
    #[error("test mode: {0}")]
    TestMode(#[from] HookError),
    /// Debug1 could not be served.
    #[error("debug bus: {0}")]
    DebugBus(#[from] IpcError),
    /// A build with Debug1 was given no bus address.
    #[error("the debug bus address is not set")]
    NoDebugBus,
    /// A build that can mint test input refused to start outside the jail.
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    #[error("{0}")]
    Jail(#[from] crate::jail_check::JailRefusal),
}

impl RunError {
    /// The process exit code this error maps to: 5 for a jail breach (11 §6.1), 1 otherwise.
    pub fn exit_code(&self) -> u8 {
        match self {
            #[cfg(any(feature = "testing", feature = "debug-inject"))]
            RunError::Jail(_) => JAIL_BREACH,
            _ => 1,
        }
    }
}

/// Runs the compositor until it is asked to stop. A build with `testing` or `debug-inject`
/// refuses every backend but headless and refuses to start outside the jail (§3.2).
pub fn run(config: RunConfig) -> Result<(), RunError> {
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    crate::jail_check::require_start(&config.start, std::path::Path::new("/"))?;
    serve(config)
}

/// Starts Debug1 on the private bus and returns the loop-side server. The connection stays in
/// `state.ipc.handles.debug`, so the name is held for as long as the compositor runs.
#[cfg(feature = "debug-inject")]
fn attach_debug(
    state: &mut State,
    handle: &smithay::reexports::calloop::LoopHandle<'static, State>,
    bus: &DebugBus,
) -> Result<debug::Shared, RunError> {
    let DebugBus::At(address) = bus else {
        return Err(RunError::NoDebugBus);
    };
    let (calls, connection) = casement_ipc::dbus::serve_debug_at(address)?;
    state.ipc.handles.debug = Some(connection);
    Ok(debug::attach(handle, calls)?)
}

/// Starts the compositor, listens for clients, and runs the event loop: a turn dispatches
/// what is ready, drains queued input into the model, reads the nested window if there is
/// one, answers the debug object, and flushes every client.
fn serve(config: RunConfig) -> Result<(), RunError> {
    let RunConfig {
        start,
        debug_bus,
        sleep_bus,
    } = config;
    let mut event_loop: EventLoop<'static, State> = EventLoop::try_new()?;
    let handle = event_loop.handle();
    let mut display: Display<State> = Display::new()?;
    let config_home = start.config_home.clone();
    let tty_mode = start.tty.clone();
    let xwayland_config = start.xwayland.clone();
    let mut state = State::new(start, &mut display, &handle)?;
    let display = attach_display(display, &handle)?;
    let socket = ListeningSocketSource::new_auto()?;
    eprintln!(
        "casement: listening on {}",
        socket.socket_name().to_string_lossy()
    );
    let socket_name = socket.socket_name().to_string_lossy().into_owned();
    handle
        .insert_source(socket, |stream, (), state: &mut State| {
            let role = casement_core::policy::ConnectionRole::Public;
            if let Err(error) = clients::insert_client(state, stream, role, PeerIdentity::Unproven)
            {
                tracing::warn!(%error, "a client could not be inserted");
            }
        })
        .map_err(LoopError::from)?;
    #[cfg(feature = "debug-inject")]
    let debug = attach_debug(&mut state, &handle, &debug_bus)?;
    #[cfg(not(feature = "debug-inject"))]
    let _ = debug_bus;
    let wayland = crate::xwayland::WaylandSocket(socket_name);
    crate::xwayland::attach(&mut state, &xwayland_config);
    // Held for the run: dropping it stops the thread that hears the machine go to sleep.
    // The vkms test mode has no logind: its pause and return are signals instead.
    let _sleep = match &tty_mode {
        cli::TtyMode::Seat => crate::suspend::attach(&mut state, sleep_bus),
        _ => None,
    };
    let mut hooks = TestHooks::attach(&mut state, &tty_mode)?;
    core_loop::announce_outputs(&mut state);
    crate::keymap::attach(&mut state, &config_home)?;
    let stop = event_loop.get_signal();
    #[cfg(feature = "debug-inject")]
    let wake = Some(debug::WAITING_TURN);
    #[cfg(not(feature = "debug-inject"))]
    let wake = hooks.wake();
    event_loop.run(wake, &mut state, |state| {
        core_loop::turn(state);
        hooks.turn(state);
        crate::xwayland::turn(state, &xwayland_config, &wayland);
        #[cfg(feature = "debug-inject")]
        debug::turn(&debug, state);
        if let BackendKind::Winit(window) = &mut state.backend
            && window.dispatch().is_break()
        {
            stop.stop();
        }
        display::flush(&display);
    })?;
    Ok(())
}
