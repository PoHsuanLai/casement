//! One output's vblank instants: a phase and a refresh rate.

use super::{Millihertz, Mono};
use std::time::Duration;

/// A vblank's number since its clock's phase: the phase itself is 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VblankSeq(pub u64);

/// When an output's vblanks fall: `phase`, then every refresh period after it. Instants are
/// computed from the phase each time (`phase + k × period`), never accumulated, so they do not
/// drift over a long run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VblankClock {
    refresh: Millihertz,
    phase: Mono,
}

impl VblankClock {
    /// A clock whose vblank 0 is at `phase`.
    pub fn new(refresh: Millihertz, phase: Mono) -> VblankClock {
        VblankClock { refresh, phase }
    }

    /// The first vblank strictly after `t`, and its number. Before the phase, that is the
    /// phase itself.
    pub fn next_after(&self, t: Mono) -> (Mono, VblankSeq) {
        if t < self.phase {
            return (self.phase, VblankSeq(0));
        }
        let elapsed = t.since(self.phase).as_nanos();
        let guess = self.refresh.frames_in(elapsed);
        let k = (guess..=guess.saturating_add(2))
            .find(|k| u128::from(self.refresh.nanos_for(*k)) > elapsed)
            .unwrap_or(guess.saturating_add(2));
        (self.at(k), VblankSeq(k))
    }

    /// Vblank `k`'s instant.
    fn at(&self, k: u64) -> Mono {
        self.phase
            .after(Duration::from_nanos(self.refresh.nanos_for(k)))
    }
}
