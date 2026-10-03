//! The virtual clock: an instant that moves only when told, the timers and the outputs'
//! vblank clocks it stops for. It mirrors ds-core's `VirtualClock` (`elapsed`, `next_due`,
//! `advance_to`, `schedule`) as a plain value: no thread-installed global, no executor
//! (21-FREEZE-AMENDMENTS A1). The composition root drives it as a calloop event source.

use super::{Due, Mono, TimerBook, TimerKey, VblankClock, VblankSeq};
use crate::ids::OutputKey;
use std::collections::BTreeMap;
use std::time::Duration;

/// Why [`VirtualClock::next_stop`] stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    /// A timer is due.
    Timer(TimerKey),
    /// An output's vblank.
    Vblank(OutputKey, VblankSeq),
    /// Nothing is due before the limit.
    End,
}

/// Everything that happens on arriving at one instant, in the order it is handled: every due
/// timer in firing order, then every output's vblank by output key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    /// The instant reached.
    pub at: Mono,
    /// The timers that fired, earliest first, ties in arming order.
    pub timers: Vec<TimerKey>,
    /// The vblanks that fell exactly at `at`, by output key.
    pub vblanks: Vec<(OutputKey, VblankSeq)>,
}

/// A clock that stands still until advanced, stopping at every due instant in order: timers
/// before vblanks at the same instant, timers by arming order, vblanks by output key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualClock {
    now: Mono,
    timers: TimerBook,
    outputs: BTreeMap<OutputKey, VblankClock>,
}

impl VirtualClock {
    /// A clock standing at `epoch` with no timers and no outputs.
    pub fn new(epoch: Mono) -> VirtualClock {
        VirtualClock {
            now: epoch,
            timers: TimerBook::empty(),
            outputs: BTreeMap::new(),
        }
    }

    /// Now on this clock.
    pub fn now(&self) -> Mono {
        self.now
    }

    /// How far past the session epoch the clock stands (ds-core's `elapsed`).
    pub fn elapsed(&self) -> Duration {
        self.now.0
    }

    /// The armed timers.
    pub fn timers(&self) -> &TimerBook {
        &self.timers
    }

    /// The clock with `timers` in place of its own.
    pub fn with_timers(self, timers: TimerBook) -> VirtualClock {
        VirtualClock { timers, ..self }
    }

    /// The clock with `due` armed (ds-core's `schedule`).
    pub fn schedule(self, due: Due) -> VirtualClock {
        let VirtualClock {
            now,
            timers,
            outputs,
        } = self;
        VirtualClock {
            now,
            timers: timers.arm(due),
            outputs,
        }
    }

    /// The clock with `key`'s vblanks following `clock`.
    pub fn with_output(self, key: OutputKey, clock: VblankClock) -> VirtualClock {
        let mut outputs = self.outputs;
        outputs.insert(key, clock);
        VirtualClock { outputs, ..self }
    }

    /// The clock without `key`'s vblanks.
    pub fn without_output(self, key: &OutputKey) -> VirtualClock {
        let mut outputs = self.outputs;
        outputs.remove(key);
        VirtualClock { outputs, ..self }
    }

    /// The earliest instant a timer is due strictly after now (ds-core's `next_due`).
    pub fn next_due(&self) -> Option<Mono> {
        self.timers.next_after(self.now)
    }

    /// Where the next advance stops, at most `limit`: a timer due now or later (one already
    /// due stops at now), else the earliest vblank strictly after now, else `limit` with
    /// [`Stop::End`]. Never before now.
    pub fn next_stop(&self, limit: Mono) -> (Mono, Stop) {
        let timer = self
            .timers
            .next()
            .map(|due| (due.at.max(self.now), Stop::Timer(due.key)));
        let vblank = self
            .outputs
            .iter()
            .map(|(key, clock)| {
                let (at, seq) = clock.next_after(self.now);
                (at, Stop::Vblank(key.clone(), seq))
            })
            .min_by_key(|(at, _)| *at);
        let first = match (timer, vblank) {
            (Some(t), Some(v)) => Some(if t.0 <= v.0 { t } else { v }),
            (t, v) => t.or(v),
        };
        match first {
            Some((at, stop)) if at <= limit => (at, stop),
            _ => (limit.max(self.now), Stop::End),
        }
    }

    /// The clock moved to `t` (never backwards). Timers due by then stay armed until
    /// [`VirtualClock::fire_due`] takes them.
    pub fn advance_to(self, t: Mono) -> VirtualClock {
        VirtualClock {
            now: self.now.max(t),
            ..self
        }
    }

    /// The clock moved to `t`, with everything that happens there: the timers due by `t` are
    /// taken (as [`VirtualClock::fire_due`]) and every vblank that falls exactly at `t` is
    /// reported. This is the whole of one stop: [`VirtualClock::next_stop`] names only the
    /// first thing at an instant, and a timer and a vblank (or two outputs' vblanks) may share
    /// it, so a driver steps with `next_stop` then `arrive`, never `advance_to` alone.
    ///
    /// Vblanks are reported only when the clock moves: one that falls at the instant an output
    /// is added, or at an instant already arrived at, is not.
    pub fn arrive(self, t: Mono) -> (VirtualClock, Arrival) {
        let moved = t > self.now;
        let clock = self.advance_to(t);
        let at = clock.now;
        let vblanks = if moved {
            clock.vblanks_at_now()
        } else {
            Vec::new()
        };
        let (clock, timers) = clock.fire_due();
        (
            clock,
            Arrival {
                at,
                timers,
                vblanks,
            },
        )
    }

    /// Every output whose vblank falls exactly now, by output key.
    fn vblanks_at_now(&self) -> Vec<(OutputKey, VblankSeq)> {
        self.outputs
            .iter()
            .filter_map(|(key, clock)| Some((key.clone(), clock.seq_at(self.now)?)))
            .collect()
    }

    /// The clock without every timer due by now, and their keys in firing order.
    pub fn fire_due(self) -> (VirtualClock, Vec<TimerKey>) {
        let (timers, fired) = self.timers.fire_until(self.now);
        (VirtualClock { timers, ..self }, fired)
    }
}
