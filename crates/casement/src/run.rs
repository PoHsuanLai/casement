//! Running the compositor: [`run`] starts casement from a [`RunConfig`] and drives its event
//! loop until it is asked to stop. The binary is only arguments, the environment and an exit
//! code around this; a test harness or a second binary embeds the compositor the same way.

use crate::cli::StartMode;
#[cfg(feature = "debug-inject")]
use crate::debug;
use crate::display::{self, attach_display};
use crate::notify::{Notice, NotifySocket, SocketNotify, Watchdog};
use crate::privacy::SystemDir;
use crate::space_sync::SpaceSync;
use crate::spawn::{ProcessSpawner, Scopes};
use crate::test_mode::{HookError, TestHooks};
use crate::{StartError, State, cli, clients, core_loop};
use casement_ipc::IpcError;
use casement_ipc::dbus::{CallerSetup, CallerTable};
use casement_ipc::privacy::{PrivacyEdit, PrivacyLink};
use casement_ipc::sleep::SleepBus;
use casement_ipc::space_registry::ZbusSpaceRegistry;
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

/// Where `org.quire.Compositor1` is served (P2-Z; the shell and the agent tier ask it for their
/// connections).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum CompositorBus {
    /// No bus address was given: no hand-out is offered.
    #[default]
    Unset,
    /// The session bus at this address.
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
    /// Where `org.quire.Compositor1` is served.
    pub compositor_bus: CompositorBus,
    /// Where the service manager is told casement is ready (`$NOTIFY_SOCKET`).
    pub notify: NotifySocket,
    /// How often the service manager wants a ping.
    pub watchdog: Watchdog,
    /// Whether the programs a binding starts get a scope of their own in the person's service
    /// manager.
    pub scopes: Scopes,
    /// Test overrides of which unit a bus peer runs in (feature `debug-inject` only).
    #[cfg(feature = "debug-inject")]
    pub debug_peer_units: casement_ipc::dbus::PeerUnitTable,
}

impl RunConfig {
    /// A run that starts as `start` says, with no debug bus.
    pub fn new(start: cli::StartConfig) -> RunConfig {
        RunConfig {
            start,
            debug_bus: DebugBus::Unset,
            sleep_bus: SleepBus::System,
            compositor_bus: CompositorBus::Unset,
            notify: NotifySocket::Unset,
            watchdog: Watchdog::Off,
            scopes: Scopes::Off,
            #[cfg(feature = "debug-inject")]
            debug_peer_units: casement_ipc::dbus::PeerUnitTable::default(),
        }
    }

    /// The same run taking bus peers to run in the units `debug_peer_units` names (a build with
    /// `debug-inject` only; the hand-out then trusts the table before `/proc`).
    #[cfg(feature = "debug-inject")]
    #[must_use]
    pub fn with_debug_peer_units(
        self,
        debug_peer_units: casement_ipc::dbus::PeerUnitTable,
    ) -> RunConfig {
        RunConfig {
            debug_peer_units,
            ..self
        }
    }

    /// The same run reading the administrator's privacy settings from `system_dir` instead of
    /// `/etc/quire` (a jailed run or a test).
    #[must_use]
    pub fn with_system_dir(self, system_dir: SystemDir) -> RunConfig {
        RunConfig {
            start: cli::StartConfig {
                system_dir,
                ..self.start
            },
            ..self
        }
    }

    /// The same run with Debug1 on `debug_bus`.
    #[must_use]
    pub fn with_debug_bus(self, debug_bus: DebugBus) -> RunConfig {
        RunConfig { debug_bus, ..self }
    }

    /// The same run serving `org.quire.Compositor1` on `compositor_bus`.
    #[must_use]
    pub fn with_compositor_bus(self, compositor_bus: CompositorBus) -> RunConfig {
        RunConfig {
            compositor_bus,
            ..self
        }
    }

    /// The same run telling the service manager through `notify`.
    #[must_use]
    pub fn with_notify(self, notify: NotifySocket) -> RunConfig {
        RunConfig { notify, ..self }
    }

    /// The same run pinging the service manager's watchdog as `watchdog` says.
    #[must_use]
    pub fn with_watchdog(self, watchdog: Watchdog) -> RunConfig {
        RunConfig { watchdog, ..self }
    }

    /// The same run starting a binding's programs in scopes as `scopes` says.
    #[must_use]
    pub fn with_scopes(self, scopes: Scopes) -> RunConfig {
        RunConfig { scopes, ..self }
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

/// Announces the test unit overrides once, plainly, when there are any (feature `debug-inject`).
#[cfg(feature = "debug-inject")]
fn announce_test_units(table: &casement_ipc::dbus::PeerUnitTable) {
    if !table.rules().is_empty() {
        tracing::warn!(
            overrides = table.rules().len(),
            "TEST ONLY: test unit overrides are active, so some programs are taken to run in units they do not"
        );
    }
}

/// Starts serving `org.quire.Compositor1` on the session bus, with the Privacy list for Settings
/// on the same connection, and checking the accessibility roots clients claim. A bus that cannot
/// be reached, or a name another casement holds, is logged and the compositor runs without a
/// hand-out: the shell then has no way to get its connection, and nothing else is lost.
fn attach_compositor(
    state: &mut State,
    handle: &smithay::reexports::calloop::LoopHandle<'static, State>,
    bus: &CompositorBus,
    table: CallerTable,
    #[cfg(feature = "debug-inject")] peer_units: casement_ipc::dbus::PeerUnitTable,
) -> Result<(), RunError> {
    let CompositorBus::At(address) = bus else {
        return Ok(());
    };
    let (link, edits) = PrivacyLink::new();
    let board = link.board();
    let setup = CallerSetup::desktop(table).with_privacy(link);
    #[cfg(feature = "debug-inject")]
    let setup = {
        announce_test_units(&peer_units);
        setup.with_test_units(peer_units)
    };
    match casement_ipc::dbus::serve_compositor_at(address, setup) {
        Ok((calls, connection)) => {
            state.ipc.handles.compositor = Some(connection);
            state.ipc.board = Some(board);
            crate::connect::attach(handle, calls)?;
            attach_edits(handle, edits)?;
            crate::a11y::attach(state, handle, address)?;
        }
        Err(error) => {
            tracing::warn!(%error, "org.quire.Compositor1 is not served: no connection can be handed out");
        }
    }
    Ok(())
}

/// Settings' changes to the Privacy list reach the model as they are made.
fn attach_edits(
    handle: &smithay::reexports::calloop::LoopHandle<'static, State>,
    edits: smithay::reexports::calloop::channel::Channel<PrivacyEdit>,
) -> Result<(), LoopError> {
    handle
        .insert_source(edits, |event, (), state: &mut State| {
            if let smithay::reexports::calloop::channel::Event::Msg(edit) = event {
                crate::privacy::edited(state, edit);
            }
        })
        .map_err(LoopError::from)?;
    Ok(())
}

/// `SIGHUP` reads the administrator's privacy settings again.
fn attach_reload(handle: &smithay::reexports::calloop::LoopHandle<'static, State>) {
    let requests = match crate::hangup::watch() {
        Ok(requests) => requests,
        Err(error) => {
            tracing::warn!(%error, "SIGHUP is not heard, so the administrator's privacy settings are read at start only");
            return;
        }
    };
    let inserted = handle.insert_source(requests, |event, (), state: &mut State| {
        if let smithay::reexports::calloop::channel::Event::Msg(_) = event {
            crate::privacy::reload_floor(state);
        }
    });
    if let Err(error) = inserted {
        tracing::warn!(%error, "SIGHUP is not heard, so the administrator's privacy settings are read at start only");
    }
}

/// Starts the compositor, listens for clients, and runs the event loop: a turn dispatches
/// what is ready, drains queued input into the model, reads the nested window if there is
/// one, answers the debug object, and flushes every client.
fn serve(config: RunConfig) -> Result<(), RunError> {
    let RunConfig {
        start,
        debug_bus,
        sleep_bus,
        compositor_bus,
        notify,
        watchdog,
        scopes,
        #[cfg(feature = "debug-inject")]
        debug_peer_units,
    } = config;
    let mut event_loop: EventLoop<'static, State> = EventLoop::try_new()?;
    let handle = event_loop.handle();
    let mut display: Display<State> = Display::new()?;
    let config_home = start.config_home.clone();
    let start_mode = start.mode;
    let tty_mode = start.tty.clone();
    let xwayland_config = start.xwayland.clone();
    let mut state = State::new(start, &mut display, &handle)?;
    state.seams.notify = Box::new(SocketNotify::new(notify));
    // The run's own spawner is the one that may use scopes; a seam a test injected is kept.
    if matches!(scopes, Scopes::Under(_)) {
        state.seams.spawner = Box::new(ProcessSpawner::new(scopes));
    }
    if let CompositorBus::At(address) = &compositor_bus {
        let registry = ZbusSpaceRegistry::at(address.clone());
        state.seams.space_sync = SpaceSync::new(Box::new(registry));
    }
    let display = attach_display(display, &handle)?;
    let socket = ListeningSocketSource::new_auto()?;
    eprintln!(
        "casement: listening on {}",
        socket.socket_name().to_string_lossy()
    );
    let socket_name = socket.socket_name().to_string_lossy().into_owned();
    state.seams.child_env =
        std::mem::take(&mut state.seams.child_env).with("WAYLAND_DISPLAY", &socket_name);
    let status = format!("listening on {socket_name}");
    // The desktop's own list of its programs is read once; each program that connects is held,
    // looked at and judged against it, so who it is is known before it asks for anything.
    let table = clients::desktop_table();
    let judge = clients::Judge::new(table.clone());
    handle
        .insert_source(socket, move |stream, (), state: &mut State| {
            let role = casement_core::policy::ConnectionRole::Public;
            let arrival = judge.arrival(&stream);
            if let Err(error) =
                clients::insert_client(state, stream, role, arrival.peer, arrival.standing)
            {
                tracing::warn!(%error, "a client could not be inserted");
            }
        })
        .map_err(LoopError::from)?;
    #[cfg(feature = "debug-inject")]
    let debug = attach_debug(&mut state, &handle, &debug_bus)?;
    #[cfg(not(feature = "debug-inject"))]
    let _ = debug_bus;
    attach_compositor(
        &mut state,
        &handle,
        &compositor_bus,
        table,
        #[cfg(feature = "debug-inject")]
        debug_peer_units,
    )?;
    // The person's book and the administrator's floor are read before the first client is
    // taken in (the loop has not run yet), then the list is published for Settings.
    crate::privacy::start(&mut state);
    if !matches!(tty_mode, cli::TtyMode::Direct(_)) {
        // The vkms test mode uses SIGHUP for something else.
        attach_reload(&handle);
    }
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
    match start_mode {
        StartMode::Normal => {
            crate::appearance::attach(&mut state, &config_home)?;
            crate::settings_live::attach(&mut state, &config_home)?;
        }
        StartMode::Safe => tracing::warn!(
            "safe mode: the settings files are ignored, there are no effects, no X11 apps, no icons and one display"
        ),
    }
    // The Spaces are loaded and the outputs announced: register them (queued, not waited for),
    // then tell the service manager the compositor is ready.
    state.seams.space_sync.sync(state.core.model.spaces());
    if let Err(error) = state.seams.notify.send(&Notice::Ready { status }) {
        tracing::warn!(%error, "the service manager could not be told casement is ready");
    }
    if let Watchdog::Every(every) = watchdog {
        crate::time::ping_every(&handle, every);
    }
    // Night light follows the local wall clock; a virtual or stepped clock run has no wall time.
    if let crate::time::ClockSource::Monotonic(_) = &state.clock {
        crate::time::tick_every_minute(&handle);
    }
    let stop = event_loop.get_signal();
    match crate::stop::watch() {
        Ok(requests) => {
            let stopper = stop.clone();
            let inserted = handle.insert_source(requests, move |event, (), _: &mut State| {
                if let smithay::reexports::calloop::channel::Event::Msg(_) = event {
                    tracing::info!("asked to stop; closing down");
                    stopper.stop();
                }
            });
            if let Err(error) = inserted {
                tracing::warn!(%error, "SIGTERM is not heard");
            }
        }
        Err(error) => tracing::warn!(%error, "SIGTERM is not heard"),
    }
    #[cfg(feature = "debug-inject")]
    let wake = Some(debug::WAITING_TURN);
    #[cfg(not(feature = "debug-inject"))]
    let wake = hooks.wake();
    let ran = event_loop.run(wake, &mut state, |state| {
        if core_loop::nested::drain(state).is_break() {
            stop.stop();
        }
        core_loop::turn(state);
        state.seams.space_sync.poll();
        hooks.turn(state);
        crate::xwayland::turn(state, &xwayland_config, &wayland);
        #[cfg(feature = "debug-inject")]
        debug::turn(&debug, state);
        display::flush(&display);
    });
    if let Err(error) = state.seams.notify.send(&Notice::Stopping) {
        tracing::warn!(%error, "the service manager could not be told casement is stopping");
    }
    Ok(ran?)
}
