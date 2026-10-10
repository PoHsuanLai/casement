//! The real session (feature `tty`): `TtySession` over libseat, a thin adapter (B7). P1-Z
//! declares it; P1-D1 fills it.

use super::session::{SessionNote, TtySession};
use super::udev::DevicePath;
use super::vt::VtNumber;
use crate::session::SessionFault;
use std::os::fd::OwnedFd;

/// The libseat session casement runs in.
#[derive(Debug)]
pub struct RealSession {
    /// The notes waiting to be handed out.
    pub pending: Vec<SessionNote>,
}

impl RealSession {
    /// Opens the seat (logind or seatd).
    pub fn open() -> Result<RealSession, SessionFault> {
        todo!("P1-D1: RealSession::open")
    }
}

impl TtySession for RealSession {
    fn open_device(&mut self, path: &DevicePath) -> Result<OwnedFd, SessionFault> {
        let _ = path;
        todo!("P1-D1: RealSession::open_device")
    }

    fn switch_vt(&mut self, vt: VtNumber) -> Result<(), SessionFault> {
        let _ = vt;
        todo!("P1-D1: RealSession::switch_vt")
    }

    fn notes(&mut self) -> Vec<SessionNote> {
        todo!("P1-D1: RealSession::notes")
    }
}
