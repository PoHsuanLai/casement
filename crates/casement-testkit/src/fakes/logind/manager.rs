//! `org.freedesktop.login1.Manager` (P1-F2): `Inhibit`, `ListSessions`, `GetSession`,
//! `GetSessionByPID`, `GetSeat`; the `PrepareForSleep` signal is sent through
//! [`FakeLogind::prepare_for_sleep`](super::FakeLogind::prepare_for_sleep).

use super::log::Recorder;
use super::{SEAT_PATH, SESSION_ID, SESSION_PATH, inhibit, object_path};
use tokio::runtime::Handle;
use zbus::fdo;
use zbus::zvariant::{OwnedFd, OwnedObjectPath};

/// The manager object. Served as a `#[zbus::interface]` by [`FakeLogind`](super::FakeLogind).
#[derive(Debug)]
pub struct Manager {
    log: Recorder,
    handle: Handle,
}

impl Manager {
    pub(super) fn new(log: Recorder, handle: Handle) -> Manager {
        Manager { log, handle }
    }
}

fn failed(error: impl std::fmt::Display) -> fdo::Error {
    fdo::Error::Failed(error.to_string())
}

#[zbus::interface(name = "org.freedesktop.login1.Manager")]
impl Manager {
    /// `Inhibit(what, who, why, mode)`: the write end of a pipe; closing it releases the
    /// inhibitor (the fake records both). Every mode counts as a delay inhibitor.
    fn inhibit(
        &self,
        what: String,
        who: String,
        why: String,
        mode: String,
    ) -> fdo::Result<OwnedFd> {
        let _ = (who, why, mode);
        inhibit::open(&self.log, &self.handle, &what)
            .map(OwnedFd::from)
            .map_err(failed)
    }

    /// `ListSessions()`: the one session, as `(id, uid, user, seat, path)`.
    fn list_sessions(&self) -> fdo::Result<Vec<(String, u32, String, String, OwnedObjectPath)>> {
        let uid = rustix::process::getuid().as_raw();
        Ok(vec![(
            SESSION_ID.to_owned(),
            uid,
            "casement".to_owned(),
            super::SEAT_ID.to_owned(),
            object_path(SESSION_PATH)?,
        )])
    }

    /// `GetSession(id)`: the one session, by its id or by `auto`.
    fn get_session(&self, id: String) -> fdo::Result<OwnedObjectPath> {
        match id.as_str() {
            SESSION_ID | "auto" => object_path(SESSION_PATH),
            other => Err(failed(format_args!("no session {other}"))),
        }
    }

    /// `GetSessionByPID(pid)`: the one session, whatever the process.
    fn get_session_by_pid(&self, pid: u32) -> fdo::Result<OwnedObjectPath> {
        let _ = pid;
        object_path(SESSION_PATH)
    }

    /// `GetSeat(id)`: the one seat.
    fn get_seat(&self, id: String) -> fdo::Result<OwnedObjectPath> {
        match id.as_str() {
            super::SEAT_ID => object_path(SEAT_PATH),
            other => Err(failed(format_args!("no seat {other}"))),
        }
    }
}

/// Which side of a suspend `PrepareForSleep` announces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SleepPhase {
    /// The machine is about to sleep (`PrepareForSleep(true)`).
    Starting,
    /// It woke (`PrepareForSleep(false)`).
    Resumed,
}

impl SleepPhase {
    /// The signal's wire argument.
    pub(super) fn starting(self) -> bool {
        matches!(self, SleepPhase::Starting)
    }

    /// The phase the wire argument names.
    pub fn from_starting(starting: bool) -> SleepPhase {
        if starting {
            SleepPhase::Starting
        } else {
            SleepPhase::Resumed
        }
    }
}
