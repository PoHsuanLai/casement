//! How fast registrations are sent: the account service lets a caller make ten new Spaces a
//! minute, so the thread sends [`PACE`] and then waits [`PACE_WINDOW`]. No clock is read; the
//! thread sleeps for what [`Pace::before_send`] says.

use std::time::Duration;

/// Registrations sent before a wait (under the service's ten a minute, with room to spare).
pub const PACE: usize = 8;

/// The wait after [`PACE`] registrations.
pub const PACE_WINDOW: Duration = Duration::from_secs(60);

/// Counts the registrations sent since the last wait.
///
/// ```
/// use casement_ipc::space_registry::{PACE, PACE_WINDOW, Pace};
///
/// let mut pace = Pace::default();
/// for _ in 0..PACE {
///     assert_eq!(pace.before_send(), None);
/// }
/// assert_eq!(pace.before_send(), Some(PACE_WINDOW));
/// assert_eq!(pace.before_send(), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pace {
    sent: usize,
}

impl Pace {
    /// How long to wait before the next send, if at all; counts that send.
    pub fn before_send(&mut self) -> Option<Duration> {
        if self.sent >= PACE {
            self.sent = 1;
            Some(PACE_WINDOW)
        } else {
            self.sent += 1;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wait_comes_before_every_batch_after_the_first() {
        let mut pace = Pace::default();
        let waits: Vec<bool> = (0..(PACE * 2 + 1))
            .map(|_| pace.before_send().is_some())
            .collect();
        let at: Vec<usize> = waits
            .iter()
            .enumerate()
            .filter_map(|(i, w)| w.then_some(i))
            .collect();
        assert_eq!(at, [PACE, PACE * 2]);
    }
}
