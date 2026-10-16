//! Taking the display devices back after the seat returns, and trying again when that fails.
//!
//! A device that cannot take master again (the session handed the seat back a moment before the
//! kernel let go of it) would stay black for good: nothing else asks again. The devices that
//! failed are remembered here, and asked again at a slow pace until one takes master.

use super::udev::DevicePath;
use casement_core::time::Mono;
use std::collections::BTreeSet;
use std::time::Duration;

/// How often a device that could not take master is asked again.
pub const RETRY_EVERY: Duration = Duration::from_secs(1);

/// Whether a device has the display master.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Master {
    /// It does, as far as is known.
    Held,
    /// It could not take it back; nothing may be drawn on it.
    Lost,
}

/// The devices that are waiting to take master again.
///
/// ```
/// use casement_backend::tty::{DevicePath, Master, MasterWatch};
/// use casement_core::time::Mono;
/// use std::time::Duration;
///
/// let card = DevicePath("/dev/dri/card0".into());
/// let mut watch = MasterWatch::default();
/// watch.lose(card.clone());
/// assert_eq!(watch.of(&card), Master::Lost);
/// // The first retry is due at once, the next one a second later.
/// assert_eq!(watch.due(Mono(Duration::ZERO)), [card.clone()]);
/// assert!(watch.due(Mono(Duration::from_millis(500))).is_empty());
/// assert_eq!(watch.due(Mono(Duration::from_secs(1))), [card.clone()]);
/// watch.regain(&card);
/// assert_eq!(watch.of(&card), Master::Held);
/// ```
#[derive(Debug, Clone, Default)]
pub struct MasterWatch {
    lost: BTreeSet<DevicePath>,
    next: Option<Mono>,
    attempts: u32,
}

impl MasterWatch {
    /// `device` could not take master (or lost it mid-run).
    pub fn lose(&mut self, device: DevicePath) {
        self.lost.insert(device);
        self.next = None;
    }

    /// `device` has master again.
    pub fn regain(&mut self, device: &DevicePath) {
        self.lost.remove(device);
        if self.lost.is_empty() {
            self.next = None;
            self.attempts = 0;
        }
    }

    /// Whether `device` has master.
    pub fn of(&self, device: &DevicePath) -> Master {
        match self.lost.contains(device) {
            true => Master::Lost,
            false => Master::Held,
        }
    }

    /// The devices to ask again at `now`: all that are waiting, once per [`RETRY_EVERY`].
    pub fn due(&mut self, now: Mono) -> Vec<DevicePath> {
        if self.lost.is_empty() || self.next.is_some_and(|next| now < next) {
            return Vec::new();
        }
        self.next = Some(now.after(RETRY_EVERY));
        self.attempts = self.attempts.saturating_add(1);
        self.lost.iter().cloned().collect()
    }

    /// How many rounds of asking there have been since the last device regained master (to log
    /// the first and then now and then, not every second).
    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}
