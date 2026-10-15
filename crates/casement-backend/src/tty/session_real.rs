//! The real session (feature `tty`): `TtySession` over libseat, a thin adapter (B7).
//!
//! libseat reports the seat changing hands through an event source; `open` inserts it into the
//! loop, where each report is queued as a [`SessionNote`] until `notes` hands the queue out.
//! logind's `PrepareForSleep` is not libseat's to report: a thread at the binary's edge hears it
//! on the system bus and pushes it through this session's [`SleepLink`] (taken once with
//! `sleep_link`); the link's ping wakes the loop, and `settled` lets the thread release logind's
//! delay inhibitor once the backend has let go of its devices (P1-T).

use super::kms::KmsError;
use super::kms_real::FdSource;
use super::session::{SessionNote, TtySession};
use super::sleep::{SleepInbox, SleepLink, sleep_channel};
use super::udev::DevicePath;
use super::vt::VtNumber;
use crate::session::SessionFault;
use rustix::fs::OFlags;
use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::{Event as SeatEvent, Session};
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::calloop::ping::make_ping;
use std::cell::RefCell;
use std::os::fd::OwnedFd;
use std::rc::Rc;

/// What the person is told when libseat could not open a seat: the reason, then the two ways to
/// get one. The group is `seat` on some distributions and `video` on others.
fn seat_refused(reason: &dyn std::fmt::Display) -> String {
    format!(
        "{reason}. casement needs a seat to draw on the screen and read the keyboard: log in from \
         a display manager or from a text console (logind then gives the login one), or run seatd \
         and join the seat or video group (log in again after joining)"
    )
}

/// The libseat session casement runs in.
#[derive(Debug)]
pub struct RealSession {
    seat: LibSeatSession,
    pending: Rc<RefCell<Vec<SessionNote>>>,
    sleep: SleepInbox,
    link: Option<SleepLink>,
}

impl RealSession {
    /// Opens the seat (logind or seatd) and has `handle`'s loop listen for it changing hands.
    pub fn open<D: 'static>(handle: &LoopHandle<'static, D>) -> Result<RealSession, SessionFault> {
        let unavailable = SessionFault::Unavailable;
        let (seat, notifier) =
            LibSeatSession::new().map_err(|error| unavailable(seat_refused(&error)))?;
        let pending = Rc::new(RefCell::new(Vec::new()));
        let queue = Rc::clone(&pending);
        handle
            .insert_source(notifier, move |event, _, _| {
                queue.borrow_mut().push(note_of(event));
            })
            .map_err(|error| unavailable(error.error.to_string()))?;
        // The ping only wakes the loop: the notes themselves wait in the link's channel.
        let (ping, wake) = make_ping().map_err(|error| unavailable(error.to_string()))?;
        handle
            .insert_source(wake, |(), _, _| {})
            .map_err(|error| unavailable(error.error.to_string()))?;
        let (link, sleep) = sleep_channel(move || ping.ping());
        Ok(RealSession {
            seat,
            pending,
            sleep,
            link: Some(link),
        })
    }

    /// A second handle on the libseat session, for the input devices to be opened through.
    pub fn seat_handle(&self) -> LibSeatSession {
        self.seat.clone()
    }

    /// The KMS seam's device opener: a second handle on this libseat session.
    pub fn fds(&self) -> SessionFds {
        SessionFds(self.seat.clone())
    }

    /// The seat's name (`seat0`), for the udev monitor.
    pub fn seat_name(&self) -> String {
        self.seat.seat()
    }
}

/// Opens display devices through the session, for [`RealKmsOpener`](super::kms_real::RealKmsOpener).
#[derive(Debug)]
pub struct SessionFds(LibSeatSession);

impl FdSource for SessionFds {
    fn fd(&mut self, path: &DevicePath) -> Result<OwnedFd, KmsError> {
        self.0
            .open(path.as_path(), OFlags::RDWR)
            .map_err(|error| KmsError::Io(std::io::Error::other(error.to_string())))
    }
}

/// What libseat's report means to the backend.
fn note_of(event: SeatEvent) -> SessionNote {
    match event {
        SeatEvent::PauseSession => SessionNote::Paused,
        SeatEvent::ActivateSession => SessionNote::Activated,
    }
}

impl TtySession for RealSession {
    fn open_device(&mut self, path: &DevicePath) -> Result<OwnedFd, SessionFault> {
        if !self.seat.is_active() {
            return Err(SessionFault::Inactive);
        }
        self.seat
            .open(path.as_path(), OFlags::RDWR)
            .map_err(|_| SessionFault::DeniedOpen(path.as_path().display().to_string()))
    }

    fn switch_vt(&mut self, vt: VtNumber) -> Result<(), SessionFault> {
        let vt = i32::try_from(vt.0)
            .map_err(|_| SessionFault::Unavailable(format!("no VT {}", vt.0)))?;
        self.seat
            .change_vt(vt)
            .map_err(|error| SessionFault::Unavailable(error.to_string()))
    }

    fn notes(&mut self) -> Vec<SessionNote> {
        let mut notes = std::mem::take(&mut *self.pending.borrow_mut());
        notes.extend(self.sleep.take());
        notes
    }

    fn settled(&mut self, note: SessionNote) {
        if note == SessionNote::Paused {
            self.sleep.settled();
        }
    }

    fn sleep_link(&mut self) -> Option<SleepLink> {
        self.link.take()
    }
}

#[cfg(test)]
mod tests {
    use super::seat_refused;

    #[test]
    fn a_refused_seat_names_the_reason_and_the_ways_to_get_one() {
        let text = seat_refused(&"no backend available");
        for part in [
            "no backend available",
            "display manager",
            "text console",
            "seatd",
            "seat or video group",
        ] {
            assert!(text.contains(part), "{part}");
        }
    }
}
