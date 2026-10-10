//! The tty session seam (P1-Z; filled by P1-D1): who opens devices, and when the seat is
//! taken away and given back.

use super::udev::DevicePath;
use super::vt::VtNumber;
use crate::session::SessionFault;
use std::os::fd::OwnedFd;

/// A change in who holds the seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionNote {
    /// Another session took the seat (a VT switch away, a suspend): stop using devices.
    Paused,
    /// The seat is back: re-read the devices and draw.
    Activated,
    /// The person asked for this VT (a `Ctrl+Alt+Fn` chord).
    VtSwitch(u32),
}

/// The session casement runs in. Real: `session_real.rs` over libseat; fake:
/// `casement_testkit::fakes::FakeSession`.
pub trait TtySession: std::fmt::Debug {
    /// Opens a device through the session (a revoked session refuses).
    fn open_device(&mut self, path: &DevicePath) -> Result<OwnedFd, SessionFault>;

    /// Asks to switch to `vt`.
    fn switch_vt(&mut self, vt: VtNumber) -> Result<(), SessionFault>;

    /// The seat changes since the last call.
    fn notes(&mut self) -> Vec<SessionNote>;
}
