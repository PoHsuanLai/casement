//! The date, in whole seconds since the Unix epoch: the stamp on a privacy answer that is
//! kept, and the name a damaged book is set aside under. It lives here because it reads the
//! system clock (row B8); a test holds the date fixed.

use casement_core::privilege::UnixSeconds;
use casement_wayland::quire::shell_permission::WallSeconds;
use std::time::{SystemTime, UNIX_EPOCH};

/// Where the date comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WallClock {
    /// The system clock.
    System,
    /// A date that never moves (tests, and a seam nobody gave a clock).
    Fixed(UnixSeconds),
}

impl WallClock {
    /// A clock that always says the epoch: what a seam nobody gave a clock starts with.
    pub fn epoch() -> WallClock {
        WallClock::Fixed(UnixSeconds(0))
    }
}

impl WallSeconds for WallClock {
    fn now(&self) -> UnixSeconds {
        match self {
            WallClock::System => UnixSeconds(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |since| since.as_secs()),
            ),
            WallClock::Fixed(at) => *at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fixed_clock_never_moves_and_the_system_clock_is_past_2020() {
        let fixed = WallClock::Fixed(UnixSeconds(1_760_000_000));
        assert_eq!(fixed.now(), fixed.now());
        assert_eq!(WallClock::epoch().now(), UnixSeconds(0));
        assert!(WallClock::System.now().0 > 1_577_836_800);
    }
}
