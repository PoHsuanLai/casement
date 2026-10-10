//! A crash-loop guard for satellite (P2-XW): at most [`MAX_STARTS`] starts in [`WINDOW`].
//!
//! A satellite that dies as soon as it starts, with a client still waiting on the socket, would
//! be started again on every loop turn. The guard remembers the recent starts and says when the
//! next one may happen. It reads no clock: the caller passes the instant.

use casement_core::time::Mono;
use std::time::Duration;

/// The most starts allowed inside one [`WINDOW`].
pub const MAX_STARTS: usize = 4;

/// The span the starts are counted in.
pub const WINDOW: Duration = Duration::from_secs(30);

/// Whether a start may happen now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Permit {
    /// Start it; the start is recorded.
    Allowed,
    /// Too many recent starts: wait until `until`.
    Held {
        /// When the next start may happen.
        until: Mono,
    },
}

/// The recent starts of one satellite.
///
/// ```
/// use casement::xwayland::{Permit, Restarts};
/// use casement_core::time::Mono;
/// use std::time::Duration;
///
/// let mut restarts = Restarts::default();
/// assert_eq!(restarts.permit(Mono(Duration::ZERO)), Permit::Allowed);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Restarts {
    starts: Vec<Mono>,
}

impl Restarts {
    /// Asks to start at `now`; an allowed start is remembered.
    pub fn permit(&mut self, now: Mono) -> Permit {
        self.starts.retain(|start| now.since(*start) < WINDOW);
        match self.starts.first() {
            Some(oldest) if self.starts.len() >= MAX_STARTS => Permit::Held {
                until: oldest.after(WINDOW),
            },
            _ => {
                self.starts.push(now);
                Permit::Allowed
            }
        }
    }
}
