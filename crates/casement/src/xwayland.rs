//! X11 clients through xwayland-satellite (A5, P1-Z; filled by P1-X1).
//!
//! Satellite is a separate process: casement opens the X11 listening sockets, starts satellite
//! with them through the injected [`Spawner`](crate::spawn::Spawner) and exports `DISPLAY` for
//! the programs it starts. Satellite talks to casement as an ordinary Wayland client, so X11
//! windows are Wayland windows with their `WM_CLASS` as the app id. The mode and the satellite
//! binary come from configuration, never from the environment.
//!
//! ```no_run
//! use casement::xwayland::{XwaylandConfig, XwaylandMode};
//!
//! let config = XwaylandConfig::satellite("xwayland-satellite");
//! assert_eq!(config.mode, XwaylandMode::Satellite);
//! ```

use crate::spawn::{EnvVar, SpawnError, Spawned, Spawner};
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
}

impl XwaylandConfig {
    /// No X11 clients.
    pub fn off() -> XwaylandConfig {
        XwaylandConfig {
            mode: XwaylandMode::Off,
            satellite: SatellitePath(PathBuf::from("xwayland-satellite")),
        }
    }

    /// X11 clients through the satellite at `path`.
    pub fn satellite(path: &str) -> XwaylandConfig {
        XwaylandConfig {
            mode: XwaylandMode::Satellite,
            satellite: SatellitePath(PathBuf::from(path)),
        }
    }
}

/// An X11 display name (`:1`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisplayName(pub String);

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

/// A running satellite.
#[derive(Debug)]
#[non_exhaustive]
pub struct Xwayland {
    /// The display it serves.
    pub display: DisplayName,
    /// The process.
    pub process: Spawned,
}

impl Xwayland {
    /// Opens the listening sockets and starts satellite as `config` says, through `spawner`.
    pub fn start(
        config: &XwaylandConfig,
        spawner: &mut dyn Spawner,
    ) -> Result<Xwayland, XwaylandError> {
        let _ = (config, spawner);
        todo!("P1-X1: Xwayland::start")
    }

    /// The variables programs started afterwards get (`DISPLAY`).
    pub fn env(&self) -> Vec<EnvVar> {
        todo!("P1-X1: Xwayland::env")
    }
}

/// Whether X11 is being served.
#[derive(Debug, Default)]
#[non_exhaustive]
pub enum XwaylandState {
    /// Not at all.
    #[default]
    Off,
    /// Satellite is running.
    Running(Xwayland),
}
