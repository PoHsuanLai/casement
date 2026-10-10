//! The real session (feature `tty`): `TtySession` over libseat, a thin adapter (B7).
//!
//! libseat reports the seat changing hands through an event source; `open` inserts it into the
//! loop, where each report is queued as a [`SessionNote`] until `notes` hands the queue out.
//! logind's `PrepareForSleep` is not libseat's to report: a suspend reaches this adapter only as
//! the pause logind makes of the session (see FINDINGS, p1-d1).

use super::kms::KmsError;
use super::kms_real::FdSource;
use super::session::{SessionNote, TtySession};
use super::udev::DevicePath;
use super::vt::VtNumber;
use crate::session::SessionFault;
use rustix::fs::OFlags;
use smithay::backend::session::libseat::LibSeatSession;
use smithay::backend::session::{Event as SeatEvent, Session};
use smithay::reexports::calloop::LoopHandle;
use std::cell::RefCell;
use std::os::fd::OwnedFd;
use std::rc::Rc;

/// The libseat session casement runs in.
#[derive(Debug)]
pub struct RealSession {
    seat: LibSeatSession,
    pending: Rc<RefCell<Vec<SessionNote>>>,
}

impl RealSession {
    /// Opens the seat (logind or seatd) and has `handle`'s loop listen for it changing hands.
    pub fn open<D: 'static>(handle: &LoopHandle<'static, D>) -> Result<RealSession, SessionFault> {
        let unavailable = SessionFault::Unavailable;
        let (seat, notifier) =
            LibSeatSession::new().map_err(|error| unavailable(error.to_string()))?;
        let pending = Rc::new(RefCell::new(Vec::new()));
        let queue = Rc::clone(&pending);
        handle
            .insert_source(notifier, move |event, _, _| {
                queue.borrow_mut().push(note_of(event));
            })
            .map_err(|error| unavailable(error.error.to_string()))?;
        Ok(RealSession { seat, pending })
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
        std::mem::take(&mut *self.pending.borrow_mut())
    }
}
