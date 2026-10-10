//! The fake logind (`org.freedesktop.login1` subset on a private bus, the `fake-logind`
//! binary): Manager `Inhibit` (a pipe fd), `PrepareForSleep`, `ListSessions`, `GetSession`;
//! Session `TakeControl`, `ReleaseControl`, `TakeDevice`, `ReleaseDevice`,
//! `PauseDeviceComplete`, `SetIdleHint` and the `Lock`, `Unlock`, `PauseDevice`, `ResumeDevice`
//! signals; Seat `SwitchTo`. One file per object: [`manager`], [`session`], [`seat`].
//!
//! The server only ever connects to the address it is given: a [`PrivateBus`] in a test, the
//! jail's private bus in a scenario. It never touches the system bus.

mod bus;
mod control;
mod inhibit;
mod log;
pub mod manager;
pub mod seat;
mod server;
pub mod session;
mod signals;

pub use bus::{PrivateBus, PrivateBusError};
pub use manager::SleepPhase;
pub use server::FakeLogind;
pub use session::{IdleHint, PauseKind};

use zbus::zvariant::OwnedObjectPath;

/// The well-known name the fake owns.
pub const BUS_NAME: &str = "org.freedesktop.login1";
/// The manager object's path.
pub const MANAGER_PATH: &str = "/org/freedesktop/login1";
/// The one session's id.
pub const SESSION_ID: &str = "c1";
/// The one session's object path.
pub const SESSION_PATH: &str = "/org/freedesktop/login1/session/c1";
/// The one seat's id.
pub const SEAT_ID: &str = "seat0";
/// The one seat's object path.
pub const SEAT_PATH: &str = "/org/freedesktop/login1/seat/seat0";
/// The control interface the fake serves for jailed scenarios (`Lock`, `Unlock`,
/// `PrepareForSleep`, `Calls`).
pub const CONTROL_INTERFACE: &str = "org.quire.Casement.FakeLogind1";
/// The control object's path.
pub const CONTROL_PATH: &str = "/org/quire/Casement/FakeLogind1";

/// A call the fake recorded, for assertions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum LogindCall {
    /// `Inhibit(what, ...)`: the `what` string.
    Inhibit(String),
    /// The inhibitor's pipe was closed.
    InhibitReleased(String),
    /// `TakeControl`.
    TakeControl,
    /// `ReleaseControl`.
    ReleaseControl,
    /// `Activate`.
    Activate,
    /// `SetIdleHint(idle)`.
    IdleHint(IdleHint),
    /// `TakeDevice(major, minor)`.
    TakeDevice(u32, u32),
    /// `ReleaseDevice(major, minor)`.
    ReleaseDevice(u32, u32),
    /// `PauseDeviceComplete(major, minor)`.
    PauseDeviceComplete(u32, u32),
    /// `SwitchTo(vt)`.
    SwitchTo(u32),
}

/// Why the fake logind could not serve or emit.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LogindError {
    /// The bus at `address` could not be reached or refused the name or objects.
    #[error("bus {address}: {source}")]
    Bus {
        /// The bus address.
        address: String,
        /// Why the connection failed.
        #[source]
        source: zbus::Error,
    },
    /// The runtime that drives the server could not start.
    #[error("runtime: {source}")]
    Runtime {
        /// Why it failed.
        #[source]
        source: std::io::Error,
    },
    /// A signal could not be sent.
    #[error("signal {member}: {source}")]
    Emit {
        /// The signal's member name.
        member: &'static str,
        /// Why it failed.
        #[source]
        source: zbus::Error,
    },
    /// A pipe could not be made or configured.
    #[error("pipe: {source}")]
    Pipe {
        /// Why it failed.
        #[source]
        source: rustix::io::Errno,
    },
    /// Inhibitors were still held when the wait ended.
    #[error("inhibitors still held: {what:?}")]
    StillHeld {
        /// The `what` of each inhibitor still held.
        what: Vec<String>,
    },
}

/// A path constant as a wire object path.
pub(crate) fn object_path(path: &'static str) -> zbus::fdo::Result<OwnedObjectPath> {
    OwnedObjectPath::try_from(path).map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
}
