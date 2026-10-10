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

/// Whether this session holds the seat. While it is [`SeatState::Lent`] no device may be used,
/// no frame is drawn and hotplug waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum SeatState {
    /// This session is on the screen.
    #[default]
    Held,
    /// Another VT, or a suspend, has the seat.
    Lent,
}

impl SeatState {
    /// The state after `note`, from `self`.
    ///
    /// ```
    /// use casement_backend::tty::{SeatState, SessionNote};
    ///
    /// assert_eq!(SeatState::Held.after(SessionNote::Paused), SeatState::Lent);
    /// assert_eq!(SeatState::Lent.after(SessionNote::Activated), SeatState::Held);
    /// assert_eq!(SeatState::Lent.after(SessionNote::VtSwitch(2)), SeatState::Lent);
    /// ```
    #[must_use]
    pub fn after(self, note: SessionNote) -> SeatState {
        match note {
            SessionNote::Paused => SeatState::Lent,
            SessionNote::Activated => SeatState::Held,
            _ => self,
        }
    }
}
