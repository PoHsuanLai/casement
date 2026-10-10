//! `org.freedesktop.login1.Manager` (P1-F2): `Inhibit`, `PrepareForSleep`, `ListSessions`.

use super::LogindError;
use std::os::fd::OwnedFd;

/// The manager object's methods. P1-F2 serves them as a `#[zbus::interface]`.
#[derive(Debug, Default)]
pub struct Manager;

impl Manager {
    /// `Inhibit(what, who, why, mode)`: the read end of a pipe whose close releases the
    /// inhibitor (the fake records both).
    pub fn inhibit(
        &mut self,
        what: &str,
        who: &str,
        why: &str,
        mode: &str,
    ) -> Result<OwnedFd, LogindError> {
        let _ = (what, who, why, mode);
        todo!("P1-F2: Manager::inhibit")
    }

    /// Emits `PrepareForSleep(true)` then, once every inhibitor is released, lets the test
    /// emit `PrepareForSleep(false)`.
    pub fn prepare_for_sleep(&mut self, starting: SleepPhase) -> Result<(), LogindError> {
        let _ = starting;
        todo!("P1-F2: Manager::prepare_for_sleep")
    }

    /// `ListSessions()`: the session ids the fake has.
    pub fn list_sessions(&self) -> Vec<String> {
        todo!("P1-F2: Manager::list_sessions")
    }
}

/// Which side of a suspend `PrepareForSleep` announces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SleepPhase {
    /// The machine is about to sleep.
    Starting,
    /// It woke.
    Resumed,
}
