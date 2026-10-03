//! A one-dimensional leg: a Space strip's offset.

use crate::cause_travel::Travel;
use crate::window_motion::{leg_to, still_leg};
use casement_core::time::Mono;
use ds_motion::spring::{Leg, SpringPhase, State};
use ds_style::appearance::motion::MotionLevel;

/// How many pixels one unit of the leg draws.
const PX_PER_UNIT: f64 = 1.0;

/// One leg and when it began.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SlideLeg {
    /// The leg (logical pixels for a strip).
    leg: Leg,
    /// When it began.
    began: Mono,
}

impl SlideLeg {
    /// A strip standing still at `at`.
    pub(crate) fn still(at: f64, now: Mono) -> SlideLeg {
        SlideLeg {
            leg: still_leg(at),
            began: now,
        }
    }

    /// Where the strip is at `now`, and how fast it moves.
    pub(crate) fn state(&self, now: Mono) -> State {
        self.leg.at(now.since(self.began))
    }

    /// Whether it still moves at `now`.
    pub(crate) fn is_moving(&self, now: Mono) -> bool {
        self.leg.phase(now.since(self.began), PX_PER_UNIT) == SpringPhase::Moving
    }

    /// Aimed at `target` from where it is at `now`: a spring keeps its speed and adds `thrown`
    /// (a released swipe's); `Snap` puts it there at once.
    pub(crate) fn aimed(
        &self,
        target: f64,
        thrown: f64,
        travel: Travel,
        level: MotionLevel,
        now: Mono,
    ) -> SlideLeg {
        let mut here = self.state(now);
        here.velocity += thrown;
        let leg = match travel.spring() {
            None => still_leg(target),
            Some(spec) => leg_to(here, target, spec, level),
        };
        SlideLeg { leg, began: now }
    }
}
