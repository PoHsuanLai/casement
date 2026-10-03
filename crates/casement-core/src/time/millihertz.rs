//! A refresh rate in thousandths of a hertz, the unit KMS modes and `wl_output.mode` use.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Nanoseconds in a second, times a thousand: a period in ns is this over the rate in mHz.
const NS_PER_S_TIMES_1000: u128 = 1_000_000_000_000;

/// A refresh rate in millihertz: `Millihertz(60_000)` is 60 Hz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Millihertz(pub u32);

impl Millihertz {
    /// The time between two refreshes, truncated to whole nanoseconds. A zero rate (an output
    /// that reports none) is read as 1 mHz rather than dividing by zero.
    pub fn period(self) -> Duration {
        Duration::from_nanos(self.nanos_for(1))
    }

    /// The nanoseconds `frames` refreshes take, computed in one division so a long run does
    /// not accumulate the period's truncation.
    pub(crate) fn nanos_for(self, frames: u64) -> u64 {
        let rate = u128::from(self.0.max(1));
        u64::try_from(u128::from(frames) * NS_PER_S_TIMES_1000 / rate).unwrap_or(u64::MAX)
    }

    /// How many whole refreshes fit in `nanos`.
    pub(crate) fn frames_in(self, nanos: u128) -> u64 {
        let rate = u128::from(self.0.max(1));
        u64::try_from(nanos * rate / NS_PER_S_TIMES_1000).unwrap_or(u64::MAX)
    }
}
