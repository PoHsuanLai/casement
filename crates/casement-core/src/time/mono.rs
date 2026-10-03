//! The session's monotonic instant.

use ds_core::time::stamp::Stamp;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// An instant: the time since the session clock's epoch, with nanosecond precision. The
/// epoch is the compositor's start (or a fixture's), so two runs given the same inputs see the
/// same instants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Mono(pub Duration);

impl Mono {
    /// `d` later (saturating at the end of time).
    pub fn after(self, d: Duration) -> Mono {
        Mono(self.0.saturating_add(d))
    }

    /// How long after `earlier` this is (zero if `earlier` is later).
    pub fn since(self, earlier: Mono) -> Duration {
        self.0.saturating_sub(earlier.0)
    }

    /// This instant in ds-core's whole milliseconds, truncated: the one conversion, made when
    /// casement feeds a ds-behaviour machine (C56).
    pub fn stamp(self) -> Stamp {
        Stamp(u64::try_from(self.0.as_millis()).unwrap_or(u64::MAX))
    }
}
