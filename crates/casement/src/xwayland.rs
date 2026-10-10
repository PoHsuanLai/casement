//! X11 clients through xwayland-satellite (A5, P1-Z; filled by P1-X1).
//!
//! Satellite is a separate process: casement opens the X11 listening sockets
//! ([`Listening`]), exports `DISPLAY` for the programs it starts, and starts satellite through
//! the injected [`Spawner`](crate::spawn::Spawner) when the first X11 client connects
//! ([`XwaylandState::serve`]). Satellite is handed the same sockets (`-listenfd`), so the
//! waiting connection is its first. Satellite talks to casement as an ordinary Wayland client,
//! so X11 windows are Wayland windows with their `WM_CLASS` as the app id. The mode and the
//! satellite binary come from configuration, never from the environment.
//!
//! ```no_run
//! use casement::xwayland::{XwaylandConfig, XwaylandMode};
//!
//! let config = XwaylandConfig::satellite("xwayland-satellite");
//! assert_eq!(config.mode, XwaylandMode::Satellite);
//! ```

mod guard;
mod listen;

pub use guard::{MAX_STARTS, Permit, Restarts, WINDOW};
pub use listen::{Listening, X11Dir};

use crate::spawn::{EnvVar, SpawnError, SpawnSpec, Spawned, Spawner};
use crate::state::State;
use casement_core::input::Argv;
use casement_core::time::Mono;
use std::os::fd::AsRawFd;
use std::path::PathBuf;

/// How X11 clients are served.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum XwaylandMode {
    /// Through xwayland-satellite.
    Satellite,
    /// Not at all: no `DISPLAY`.
    Off,
}

/// How the X server behind satellite draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum SatelliteRendering {
    /// Satellite's own choice (graphics hardware when it can).
    #[default]
    Automatic,
    /// Without graphics acceleration (`-glamor none`): a machine or a jail with no usable
    /// render node. Note satellite's spelling: `off` makes it panic (SP6).
    Software,
}

/// The satellite binary: a path, or a name looked up in `PATH`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SatellitePath(pub PathBuf);

/// The X11 configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct XwaylandConfig {
    /// How X11 clients are served.
    pub mode: XwaylandMode,
    /// The satellite binary.
    pub satellite: SatellitePath,
    /// How the X server behind satellite draws.
    pub rendering: SatelliteRendering,
}

impl XwaylandConfig {
    /// No X11 clients.
    pub fn off() -> XwaylandConfig {
        XwaylandConfig {
            mode: XwaylandMode::Off,
            satellite: SatellitePath(PathBuf::from("xwayland-satellite")),
            rendering: SatelliteRendering::Automatic,
        }
    }

    /// X11 clients through the satellite at `path`.
    pub fn satellite(path: &str) -> XwaylandConfig {
        XwaylandConfig {
            mode: XwaylandMode::Satellite,
            satellite: SatellitePath(PathBuf::from(path)),
            rendering: SatelliteRendering::Automatic,
        }
    }

    /// The same, drawing without graphics acceleration.
    #[must_use]
    pub fn with_rendering(self, rendering: SatelliteRendering) -> XwaylandConfig {
        XwaylandConfig { rendering, ..self }
    }
}

/// An X11 display name (`:1`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisplayName(pub String);

/// The name of the Wayland socket satellite connects to (`wayland-1`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WaylandSocket(pub String);

/// Why X11 could not be served.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum XwaylandError {
    /// No X11 display number was free.
    #[error("no free X11 display")]
    NoDisplay,
    /// A listening socket could not be made.
    #[error("x11 socket: {0}")]
    Socket(#[from] std::io::Error),
    /// Satellite did not start.
    #[error("xwayland-satellite: {0}")]
    Spawn(#[from] SpawnError),
}

/// A running satellite and the sockets it serves.
#[derive(Debug)]
#[non_exhaustive]
pub struct Xwayland {
    /// The process.
    pub process: Spawned,
    sockets: Listening,
}

impl Xwayland {
    /// Starts satellite as `config` says through `spawner`, serving `sockets`, connected to
    /// the Wayland socket `wayland`. Satellite's command line is
    /// `<binary> <display> -listenfd <fd>...`, the numbers being those the program inherits.
    pub fn start(
        config: &XwaylandConfig,
        spawner: &mut dyn Spawner,
        sockets: Listening,
        wayland: &WaylandSocket,
    ) -> Result<Xwayland, (XwaylandError, Listening)> {
        match spawn_satellite(config, spawner, &sockets, wayland) {
            Ok(process) => Ok(Xwayland { process, sockets }),
            Err(error) => Err((error, sockets)),
        }
    }

    /// The display it serves.
    pub fn display(&self) -> &DisplayName {
        self.sockets.display()
    }

    /// The variables programs started afterwards get (`DISPLAY`).
    pub fn env(&self) -> Vec<EnvVar> {
        display_env(self.sockets.display())
    }
}

/// The spawn of one satellite for `sockets`.
fn spawn_satellite(
    config: &XwaylandConfig,
    spawner: &mut dyn Spawner,
    sockets: &Listening,
    wayland: &WaylandSocket,
) -> Result<Spawned, XwaylandError> {
    let passed = sockets.duplicate()?;
    let mut argv = vec![
        config.satellite.0.to_string_lossy().into_owned(),
        sockets.display().0.clone(),
    ];
    for fd in &passed {
        argv.push("-listenfd".to_owned());
        argv.push(fd.as_raw_fd().to_string());
    }
    if config.rendering == SatelliteRendering::Software {
        argv.extend(["-glamor".to_owned(), "none".to_owned()]);
    }
    let spec = passed
        .into_iter()
        .fold(SpawnSpec::new(Argv(argv)), SpawnSpec::with_fd)
        .with_env("WAYLAND_DISPLAY", &wayland.0);
    Ok(spawner.spawn(spec)?)
}

/// `DISPLAY` for `display`.
fn display_env(display: &DisplayName) -> Vec<EnvVar> {
    vec![EnvVar {
        name: "DISPLAY".to_owned(),
        value: display.0.clone(),
    }]
}

/// What a call to [`XwaylandState::serve`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ServeStep {
    /// Nothing to do: off, no client waiting, or satellite already running.
    Idle,
    /// A client was waiting and satellite was started.
    Started(Spawned),
    /// A client is waiting but satellite started too often lately (a crash loop): the next
    /// start is allowed at `until`. The client keeps waiting; nothing is lost.
    Held {
        /// When the next start may happen.
        until: Mono,
    },
}

/// Whether X11 is being served.
#[derive(Debug, Default)]
#[non_exhaustive]
pub enum XwaylandState {
    /// Not at all.
    #[default]
    Off,
    /// The sockets are bound and `DISPLAY` is exported; satellite starts on the first client.
    Listening(Listening),
    /// Satellite is running.
    Running(Xwayland),
}

impl XwaylandState {
    /// The state `config` asks for at start: off, or listening on a free display in `dir`.
    pub fn bind(config: &XwaylandConfig, dir: &X11Dir) -> Result<XwaylandState, XwaylandError> {
        match config.mode {
            XwaylandMode::Satellite => Ok(XwaylandState::Listening(Listening::bind(dir)?)),
            XwaylandMode::Off => Ok(XwaylandState::Off),
        }
    }

    /// Starts satellite when a client is waiting on the sockets, at most [`MAX_STARTS`] times in
    /// [`WINDOW`] (`now` is the session clock's); a start that fails leaves the sockets bound
    /// (the next call tries again) and returns why.
    pub fn serve(
        &mut self,
        config: &XwaylandConfig,
        spawner: &mut dyn Spawner,
        wayland: &WaylandSocket,
        now: Mono,
    ) -> Result<ServeStep, XwaylandError> {
        let XwaylandState::Listening(waiting) = self else {
            return Ok(ServeStep::Idle);
        };
        if !waiting.connection_waiting() {
            return Ok(ServeStep::Idle);
        }
        if let Permit::Held { until } = waiting.restarts_mut().permit(now) {
            return Ok(ServeStep::Held { until });
        }
        let XwaylandState::Listening(sockets) = std::mem::take(self) else {
            return Ok(ServeStep::Idle);
        };
        match Xwayland::start(config, spawner, sockets, wayland) {
            Ok(running) => {
                let step = ServeStep::Started(running.process);
                *self = XwaylandState::Running(running);
                Ok(step)
            }
            Err((error, sockets)) => {
                *self = XwaylandState::Listening(sockets);
                Err(error)
            }
        }
    }

    /// Satellite exited: back to listening, so the next client starts it again.
    pub fn satellite_exited(&mut self) {
        if let XwaylandState::Running(running) = std::mem::take(self) {
            *self = XwaylandState::Listening(running.sockets);
        }
    }

    /// The display being served, when X11 is on.
    pub fn display(&self) -> Option<&DisplayName> {
        match self {
            XwaylandState::Off => None,
            XwaylandState::Listening(sockets) => Some(sockets.display()),
            XwaylandState::Running(running) => Some(running.display()),
        }
    }

    /// The variables programs started afterwards get: `DISPLAY` from the moment the sockets
    /// are bound, nothing when off.
    pub fn env(&self) -> Vec<EnvVar> {
        match self {
            XwaylandState::Off => Vec::new(),
            XwaylandState::Listening(sockets) => display_env(sockets.display()),
            XwaylandState::Running(running) => running.env(),
        }
    }

    /// Whether the running satellite has exited; if so the sockets are listening again. Only a
    /// child of this process is seen (`waitpid`); a pid that is not ours (a test double's) is
    /// never reported as exited.
    pub fn reap(&mut self) {
        let XwaylandState::Running(running) = &*self else {
            return;
        };
        let pid = i32::try_from(running.process.pid.0)
            .ok()
            .and_then(rustix::process::Pid::from_raw);
        let exited = pid.is_some_and(|pid| {
            matches!(
                rustix::process::waitpid(Some(pid), rustix::process::WaitOptions::NOHANG),
                Ok(Some(_))
            )
        });
        if exited {
            tracing::warn!("xwayland-satellite exited; it starts again with the next X11 client");
            self.satellite_exited();
        }
    }
}

/// The directory X11 path sockets live in on a real system.
const SYSTEM_X11_DIR: &str = "/tmp/.X11-unix";

/// Binds the X11 sockets as `config` asks, at start (`DISPLAY` is exported from here on). A
/// failure is logged and X11 stays off: the session works without it.
pub fn attach(state: &mut State, config: &XwaylandConfig) {
    match XwaylandState::bind(config, &X11Dir(PathBuf::from(SYSTEM_X11_DIR))) {
        Ok(bound) => {
            if let Some(display) = bound.display() {
                eprintln!("casement: X11 display {}", display.0);
            }
            state.xwayland = bound;
        }
        Err(error) => tracing::warn!(%error, "X11 clients are not served"),
    }
}

/// One loop turn of X11: notices a satellite that exited, and starts one when a client waits.
pub fn turn(state: &mut State, config: &XwaylandConfig, wayland: &WaylandSocket) {
    state.xwayland.reap();
    let State {
        xwayland,
        seams,
        clock,
        ..
    } = state;
    let now = clock.now();
    match xwayland.serve(config, seams.spawner.as_mut(), wayland, now) {
        Ok(ServeStep::Started(process)) => {
            tracing::info!(pid = process.pid.0, "xwayland-satellite started");
        }
        Ok(ServeStep::Held { until }) => {
            tracing::debug!(
                ?until,
                "xwayland-satellite keeps exiting; holding its restart"
            );
        }
        Ok(ServeStep::Idle) => {}
        Err(error) => tracing::warn!(%error, "xwayland-satellite did not start"),
    }
}
