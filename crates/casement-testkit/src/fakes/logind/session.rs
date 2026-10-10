//! `org.freedesktop.login1.Session` (P1-F2): `TakeControl`, `ReleaseControl`, `Activate`,
//! `SetIdleHint`, `TakeDevice`, `ReleaseDevice`, `PauseDeviceComplete` as recorders; the
//! `Lock`, `Unlock`, `PauseDevice` and `ResumeDevice` signals are sent through
//! [`FakeLogind`](super::FakeLogind).

use super::log::Recorder;
use super::{LogindCall, LogindError, SEAT_ID, SEAT_PATH, SESSION_ID, inhibit, object_path};
use std::os::fd::OwnedFd;
use zbus::fdo;
use zbus::zvariant::{OwnedFd as WireFd, OwnedObjectPath};

/// The session object. Served as a `#[zbus::interface]` by [`FakeLogind`](super::FakeLogind).
#[derive(Debug)]
pub struct Session {
    log: Recorder,
}

impl Session {
    pub(super) fn new(log: Recorder) -> Session {
        Session { log }
    }

    /// What `TakeDevice(major, minor)` returns: a descriptor for a fixture device (the read end of a pipe
    /// whose write end is already closed), recorded.
    pub fn open_device(&self, major: u32, minor: u32) -> Result<OwnedFd, LogindError> {
        let (read, _write) = inhibit::pipe()?;
        self.log.record(LogindCall::TakeDevice(major, minor));
        Ok(read)
    }
}

#[zbus::interface(name = "org.freedesktop.login1.Session")]
impl Session {
    /// `TakeControl(force)`, recorded. The wire carries a bool.
    fn take_control(&self, force: bool) {
        let _ = force;
        self.log.record(LogindCall::TakeControl);
    }

    /// `ReleaseControl()`, recorded.
    fn release_control(&self) {
        self.log.record(LogindCall::ReleaseControl);
    }

    /// `Activate()`, recorded.
    fn activate(&self) {
        self.log.record(LogindCall::Activate);
    }

    /// `SetIdleHint(idle)`, recorded.
    fn set_idle_hint(&self, idle: bool) {
        let hint = if idle {
            IdleHint::Idle
        } else {
            IdleHint::Active
        };
        self.log.record(LogindCall::IdleHint(hint));
    }

    /// `TakeDevice(major, minor)`: the descriptor and "not paused".
    fn take_device(&self, major: u32, minor: u32) -> fdo::Result<(WireFd, bool)> {
        let fd = self
            .open_device(major, minor)
            .map_err(|error| fdo::Error::Failed(error.to_string()))?;
        Ok((WireFd::from(fd), false))
    }

    /// `ReleaseDevice(major, minor)`, recorded.
    fn release_device(&self, major: u32, minor: u32) {
        self.log.record(LogindCall::ReleaseDevice(major, minor));
    }

    /// `PauseDeviceComplete(major, minor)`, recorded: the answer to a `PauseDevice` signal.
    fn pause_device_complete(&self, major: u32, minor: u32) {
        self.log
            .record(LogindCall::PauseDeviceComplete(major, minor));
    }

    #[zbus(property)]
    fn id(&self) -> String {
        SESSION_ID.to_owned()
    }

    #[zbus(property)]
    fn seat(&self) -> fdo::Result<(String, OwnedObjectPath)> {
        Ok((SEAT_ID.to_owned(), object_path(SEAT_PATH)?))
    }

    #[zbus(property)]
    fn active(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn state(&self) -> String {
        "active".to_owned()
    }

    #[zbus(property, name = "Type")]
    fn session_type(&self) -> String {
        "wayland".to_owned()
    }

    #[zbus(property)]
    fn class(&self) -> String {
        "user".to_owned()
    }

    #[zbus(property, name = "VTNr")]
    fn vtnr(&self) -> u32 {
        1
    }
}

/// What `SetIdleHint` was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IdleHint {
    /// The session is idle.
    Idle,
    /// It is not.
    Active,
}

/// The `type` of a `PauseDevice` signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PauseKind {
    /// Pause, then the session answers `PauseDeviceComplete`.
    Pause,
    /// Paused already; no answer needed.
    Force,
    /// The device is gone.
    Gone,
}

impl PauseKind {
    /// The signal's wire argument.
    pub(super) fn wire(self) -> &'static str {
        match self {
            PauseKind::Pause => "pause",
            PauseKind::Force => "force",
            PauseKind::Gone => "gone",
        }
    }
}
