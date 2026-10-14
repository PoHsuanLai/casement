//! Saying a repeating fault once: the first occurrence is reported, the repeats within a window
//! are counted, and the window's end reports how many were held back. A frame that fails at
//! every vblank (60 or more a second) would otherwise fill the disk.
//!
//! Time is an argument (CONVENTIONS addendum 6): the caller passes the session clock's `now`.
//!
//! ```
//! use casement_core::frame::{Repeat, RepeatGate};
//! use casement_core::time::Mono;
//! use std::time::Duration;
//!
//! let mut gate = RepeatGate::new(Duration::from_secs(10));
//! let at = |s| Mono(Duration::from_secs(s));
//! assert_eq!(gate.admit("HDMI-A-1", at(0)), Repeat::Report { suppressed: 0 });
//! assert_eq!(gate.admit("HDMI-A-1", at(1)), Repeat::Quiet);
//! assert_eq!(gate.admit("HDMI-A-1", at(11)), Repeat::Report { suppressed: 1 });
//! ```

use crate::time::Mono;
use std::collections::BTreeMap;
use std::time::Duration;

/// Whether to report a fault now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Repeat {
    /// Report it, and say how many identical ones were held back since the last report.
    Report {
        /// The occurrences not reported since the last report.
        suppressed: u64,
    },
    /// Hold it back: counted, to be reported with the next report.
    Quiet,
}

/// What is known of one key.
#[derive(Debug, Clone, Copy)]
struct Seen {
    last_report: Mono,
    suppressed: u64,
}

/// A gate per key (an output, a surface): one report per `every`, however often the fault
/// repeats.
#[derive(Debug, Clone)]
pub struct RepeatGate<K: Ord> {
    every: Duration,
    seen: BTreeMap<K, Seen>,
}

impl<K: Ord> RepeatGate<K> {
    /// A gate that lets one report per key through every `every`.
    pub fn new(every: Duration) -> RepeatGate<K> {
        RepeatGate {
            every,
            seen: BTreeMap::new(),
        }
    }

    /// The fault `key` happened at `now`: report it, or count it.
    pub fn admit(&mut self, key: K, now: Mono) -> Repeat {
        match self.seen.get_mut(&key) {
            Some(seen) if now.since(seen.last_report) < self.every => {
                seen.suppressed = seen.suppressed.saturating_add(1);
                Repeat::Quiet
            }
            Some(seen) => {
                let suppressed = std::mem::take(&mut seen.suppressed);
                seen.last_report = now;
                Repeat::Report { suppressed }
            }
            None => {
                self.seen.insert(
                    key,
                    Seen {
                        last_report: now,
                        suppressed: 0,
                    },
                );
                Repeat::Report { suppressed: 0 }
            }
        }
    }

    /// `key` works again: its next fault is a first one.
    pub fn clear(&mut self, key: &K) {
        self.seen.remove(key);
    }
}
