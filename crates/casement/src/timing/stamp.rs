//! A moment on `CLOCK_MONOTONIC`, in nanoseconds.

use casement_core::time::Mono;
use std::fmt;
use std::time::Duration;

/// A reading of `CLOCK_MONOTONIC` in nanoseconds: the number a script reads with
/// `time.clock_gettime_ns(time.CLOCK_MONOTONIC)`. `Stamp(0)` is "no such moment" (a line says 0
/// where an input did not cause a change).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Stamp(pub u64);

impl Stamp {
    /// No moment.
    pub const NONE: Stamp = Stamp(0);

    /// The reading at `at` on the session clock, whose zero read `epoch` (the clock's
    /// `monotonic_epoch_ns`). The session's `Mono` counts from a monotonic epoch, so the two
    /// differ by a constant and this is a sum, not a second clock read. A virtual clock has
    /// epoch 0.
    pub fn of(epoch: Stamp, at: Mono) -> Stamp {
        let nanos = u64::try_from(at.0.as_nanos()).unwrap_or(u64::MAX);
        Stamp(epoch.0.saturating_add(nanos))
    }

    /// Whether this moment is at most `window` after `earlier` (and not before it).
    pub fn within(self, earlier: Stamp, window: Duration) -> bool {
        self.0 >= earlier.0 && u128::from(self.0 - earlier.0) <= window.as_nanos()
    }

    /// The later of the two.
    #[must_use]
    pub fn or_later(self, other: Stamp) -> Stamp {
        self.max(other)
    }
}

impl fmt::Display for Stamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stamp_is_the_epoch_plus_the_session_instant() {
        let at = Mono(Duration::from_millis(1500));
        assert_eq!(Stamp::of(Stamp(7_000_000_000), at), Stamp(8_500_000_000));
        assert_eq!(Stamp::of(Stamp::NONE, at), Stamp(1_500_000_000));
        assert_eq!(Stamp::of(Stamp(u64::MAX), at), Stamp(u64::MAX));
    }

    #[test]
    fn within_counts_forward_from_the_earlier_moment_only() {
        let window = Duration::from_millis(250);
        // name, moment, earlier, within
        let cases = [
            ("same instant", 1_000, 1_000, true),
            ("edge of the window", 250_001_000, 1_000, true),
            ("past the window", 250_001_001, 1_000, false),
            ("before it", 999, 1_000, false),
        ];
        for (name, moment, earlier, want) in cases {
            assert_eq!(Stamp(moment).within(Stamp(earlier), window), want, "{name}");
        }
    }
}
