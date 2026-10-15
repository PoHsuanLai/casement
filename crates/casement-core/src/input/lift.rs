//! The vertical swipe's machine: Mission Control follows the fingers and, on lift, opens (or
//! closes) when the distance or the speed says so (03 section 4.2: the same shape as the Space
//! swipe, committing at half the distance).
//!
//! `travelled` is how far the fingers went toward the intent (up to open, down to close), clamped
//! to `0..=FULL_PX`; a full travel is 400 logical pixels, the Space swipe's page, so the numbers
//! match: commit past half unless a reverse flick of 1.5 per second, or under half on a forward
//! flick of the same speed (`ds_behaviour::space_swipe::commit`). The speed is the slope between
//! the oldest and newest position in the last 80 ms.

use ds_behaviour::space_swipe::{PageMilli, PagesPerSecMilli, commit};
use ds_core::time::stamp::Stamp;

/// How far the fingers go for Mission Control to be fully open, in logical pixels.
pub const FULL_PX: i32 = 400;

/// How far back from the lift a position still says how fast the fingers were going.
const WINDOW_MS: u64 = 80;

/// What the swipe is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// Mission Control is closed and the fingers go up to open it.
    Open,
    /// Mission Control is up and the fingers go down to close it.
    Close,
}

/// The vertical swipe.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Lift {
    /// No vertical swipe is going.
    #[default]
    Idle,
    /// The fingers are down.
    Tracking {
        /// What they are for.
        intent: Intent,
        /// Thousandths of the full travel made toward the intent, `0..=1000`.
        travelled: i32,
        /// Recent `(time, travelled)` pairs, for the speed at lift.
        samples: Vec<(Stamp, i32)>,
    },
}

/// What moves the swipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiftIn {
    /// The fingers locked vertical.
    Began(Intent),
    /// The fingers moved `dy` logical pixels, positive downward.
    Moved {
        /// The move.
        dy: i32,
    },
    /// The fingers lifted.
    Ended,
    /// The swipe was abandoned.
    Cancelled,
}

/// What the swipe wants done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiftOut {
    /// Mission Control is this open, in thousandths (0 closed, 1000 fully open).
    Progress(PageMilli),
    /// The fingers lifted: this is where it ends.
    Done(Verdict),
}

/// How a lifted swipe ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Mission Control opens.
    Open,
    /// Mission Control closes.
    Close,
    /// Nothing changes: Mission Control stays as it was before the swipe.
    Stay,
}

impl Lift {
    /// The swipe after `input` at `at`, and what it wants done.
    ///
    /// ```
    /// use casement_core::input::lift::{Intent, Lift, LiftIn, LiftOut, Verdict};
    /// use ds_core::time::stamp::Stamp;
    ///
    /// let (lift, _) = Lift::Idle.step(LiftIn::Began(Intent::Open), Stamp(0));
    /// let (lift, _) = lift.step(LiftIn::Moved { dy: -300 }, Stamp(100));
    /// let (_, out) = lift.step(LiftIn::Ended, Stamp(500));
    /// assert!(out.contains(&LiftOut::Done(Verdict::Open)));
    /// ```
    pub fn step(self, input: LiftIn, at: Stamp) -> (Lift, Vec<LiftOut>) {
        match (self, input) {
            (_, LiftIn::Began(intent)) => (
                Lift::Tracking {
                    intent,
                    travelled: 0,
                    samples: Vec::new(),
                },
                vec![LiftOut::Progress(openness(intent, 0))],
            ),
            (
                Lift::Tracking {
                    intent,
                    travelled,
                    samples,
                },
                LiftIn::Moved { dy },
            ) => follow(intent, travelled, samples, dy, at),
            (
                Lift::Tracking {
                    intent,
                    travelled,
                    samples,
                },
                LiftIn::Ended,
            ) => {
                let speed = lift_speed(&samples, at);
                let verdict = verdict(intent, travelled, speed);
                (Lift::Idle, vec![LiftOut::Done(verdict)])
            }
            (Lift::Tracking { .. }, LiftIn::Cancelled) => {
                (Lift::Idle, vec![LiftOut::Done(Verdict::Stay)])
            }
            (Lift::Idle, _) => (Lift::Idle, Vec::new()),
        }
    }
}

/// The fingers moved `dy` pixels downward.
fn follow(
    intent: Intent,
    travelled: i32,
    samples: Vec<(Stamp, i32)>,
    dy: i32,
    at: Stamp,
) -> (Lift, Vec<LiftOut>) {
    let toward = match intent {
        Intent::Open => dy.saturating_neg(),
        Intent::Close => dy,
    };
    let moved = i64::from(toward) * 1000 / i64::from(FULL_PX);
    let travelled = i32::try_from((i64::from(travelled) + moved).clamp(0, 1000)).unwrap_or(0);
    let samples = samples
        .into_iter()
        .filter(|(seen, _)| at.since(*seen) <= WINDOW_MS)
        .chain([(at, travelled)])
        .collect();
    (
        Lift::Tracking {
            intent,
            travelled,
            samples,
        },
        vec![LiftOut::Progress(openness(intent, travelled))],
    )
}

/// How open Mission Control is after `travelled` thousandths toward `intent`.
fn openness(intent: Intent, travelled: i32) -> PageMilli {
    match intent {
        Intent::Open => PageMilli(travelled),
        Intent::Close => PageMilli(1000 - travelled),
    }
}

/// The speed toward the intent at `at`, in thousandths of the full travel per second.
fn lift_speed(samples: &[(Stamp, i32)], at: Stamp) -> PagesPerSecMilli {
    let recent: Vec<&(Stamp, i32)> = samples
        .iter()
        .filter(|(seen, _)| at.since(*seen) <= WINDOW_MS)
        .collect();
    match (recent.first(), recent.last()) {
        (Some((first_at, first)), Some((last_at, last))) => {
            let ms = i64::try_from(last_at.since(*first_at)).unwrap_or(0);
            match ms {
                0 => PagesPerSecMilli(0),
                _ => PagesPerSecMilli(
                    i32::try_from((i64::from(*last) - i64::from(*first)) * 1000 / ms).unwrap_or(0),
                ),
            }
        }
        _ => PagesPerSecMilli(0),
    }
}

/// How a lift after `travelled` thousandths toward `intent` at `speed` ends.
fn verdict(intent: Intent, travelled: i32, speed: PagesPerSecMilli) -> Verdict {
    match (commit(PageMilli(travelled), speed), intent) {
        (0, _) => Verdict::Stay,
        (_, Intent::Open) => Verdict::Open,
        (_, Intent::Close) => Verdict::Close,
    }
}
