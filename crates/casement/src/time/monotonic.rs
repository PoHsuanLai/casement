//! `CLOCK_MONOTONIC` as a number, for the timing lines (`crate::timing`). It lives here with the
//! other clock readers (row B8).

use rustix::time::{ClockId, clock_gettime};

/// What `CLOCK_MONOTONIC` reads now, in nanoseconds: the clock `Instant` is, and the one
/// `time.clock_gettime_ns(time.CLOCK_MONOTONIC)` reads in a script.
pub fn monotonic_ns() -> u64 {
    let now = clock_gettime(ClockId::Monotonic);
    let seconds = u64::try_from(now.tv_sec).unwrap_or(0);
    let nanos = u64::try_from(now.tv_nsec).unwrap_or(0);
    seconds.saturating_mul(1_000_000_000).saturating_add(nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reading_is_the_systems_and_never_goes_back() {
        let (first, second) = (monotonic_ns(), monotonic_ns());
        assert!(first > 0, "a running system has been up for a while");
        assert!(second >= first);
    }
}
