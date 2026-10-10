//! `org.freedesktop.login1.Session` (P1-F2): `Lock`, `Unlock`, `SetIdleHint`, `TakeDevice`.

use super::LogindError;
use std::os::fd::OwnedFd;

/// The session object's methods. P1-F2 serves them as a `#[zbus::interface]`.
#[derive(Debug, Default)]
pub struct Session;

impl Session {
    /// Emits the `Lock` signal (the person asked to lock).
    pub fn lock(&mut self) -> Result<(), LogindError> {
        todo!("P1-F2: Session::lock")
    }

    /// Emits the `Unlock` signal.
    pub fn unlock(&mut self) -> Result<(), LogindError> {
        todo!("P1-F2: Session::unlock")
    }

    /// `SetIdleHint(idle)`, recorded.
    pub fn set_idle_hint(&mut self, hint: IdleHint) -> Result<(), LogindError> {
        let _ = hint;
        todo!("P1-F2: Session::set_idle_hint")
    }

    /// `TakeDevice(major, minor)`: a descriptor for a fixture device, recorded.
    pub fn take_device(&mut self, major: u32, minor: u32) -> Result<OwnedFd, LogindError> {
        let _ = (major, minor);
        todo!("P1-F2: Session::take_device")
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
