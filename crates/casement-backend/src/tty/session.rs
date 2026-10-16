//! The tty session seam (P1-Z; filled by P1-D1): who opens devices, and when the seat is
//! taken away and given back.

use super::sleep::SleepLink;
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

    /// The seat changes since the last call: another session taking it and giving it back.
    fn notes(&mut self) -> Vec<SessionNote>;

    /// The machine going to sleep and waking since the last call (`Paused`, `Activated`). They
    /// are kept apart from [`notes`](TtySession::notes) because the two cause lend the seat
    /// separately: the machine waking does not give back a seat another session still holds.
    /// A session whose only source of change is one stream reports it in `notes` and none here.
    fn sleep_notes(&mut self) -> Vec<SessionNote> {
        Vec::new()
    }

    /// The backend acted on `note` (it released its devices for a `Paused`): a session that
    /// holds a suspend open until then lets it go on. Nothing to do by default.
    fn settled(&mut self, note: SessionNote) {
        let _ = note;
    }

    /// The link a thread at the edge pushes logind's `PrepareForSleep` through (taken once;
    /// none for a session that has no suspend to hear).
    fn sleep_link(&mut self) -> Option<SleepLink> {
        None
    }
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

/// Where a seat change came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SeatSource {
    /// Another session took the seat or gave it back (a VT switch).
    Session,
    /// The machine went to sleep or woke.
    Sleep,
}

/// Why the seat is lent: each cause alone, so that one ending does not give back a seat the
/// other still holds away. A VT switched away to during a suspend is still away when the machine
/// wakes.
///
/// ```
/// use casement_backend::tty::{SeatCauses, SeatSource, SeatState, SessionNote};
///
/// let away = SeatCauses::default().after(SeatSource::Session, SessionNote::Paused);
/// let asleep = away.after(SeatSource::Sleep, SessionNote::Paused);
/// let woke = asleep.after(SeatSource::Sleep, SessionNote::Activated);
/// assert_eq!(woke.seat(), SeatState::Lent, "the other session still has it");
/// let back = woke.after(SeatSource::Session, SessionNote::Activated);
/// assert_eq!(back.seat(), SeatState::Held);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SeatCauses {
    session: SeatState,
    sleep: SeatState,
}

impl SeatCauses {
    /// The causes after `note` from `source`.
    #[must_use]
    pub fn after(self, source: SeatSource, note: SessionNote) -> SeatCauses {
        match source {
            SeatSource::Session => SeatCauses {
                session: self.session.after(note),
                ..self
            },
            SeatSource::Sleep => SeatCauses {
                sleep: self.sleep.after(note),
                ..self
            },
        }
    }

    /// Whether this session holds the seat: only when no cause lends it.
    pub fn seat(self) -> SeatState {
        match (self.session, self.sleep) {
            (SeatState::Held, SeatState::Held) => SeatState::Held,
            _ => SeatState::Lent,
        }
    }
}
