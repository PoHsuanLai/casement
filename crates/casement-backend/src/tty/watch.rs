//! Is a display still showing frames? The rules that tell a flip that never lands, a run of
//! failing frames and a display that has shown nothing for good, from the instants the
//! caller gives (this file reads no clock).
//!
//! A flip normally lands within a refresh. When it does not (a lost page-flip event, a GPU that
//! reset, a cable that dropped the link) nothing in the kernel says so, and the output would
//! stay frozen with the loop turning happily. [`FlipWatch`] gives the flip a deadline;
//! [`FailureRun`] counts frames that fail one after another.

use casement_core::time::Mono;
use std::time::Duration;

/// How long a queued flip may take to land before it is taken as lost. Far above a refresh at
/// the slowest rate a display has, far below what a person waits for a screen.
pub const FLIP_DEADLINE: Duration = Duration::from_millis(500);

/// How long an output may go without any flip landing, while frames are being asked for, before
/// the display counts as frozen: the watchdog is then not fed, so the service manager restarts
/// the session instead of leaving a dead screen.
pub const STARVED_AFTER: Duration = Duration::from_secs(10);

/// How many frames in a row may fail before the output is set up again.
pub const FAILURES_ALLOWED: u32 = 3;

/// One output's flips, by the instants they were queued at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlipWatch {
    /// When the flip in flight was queued.
    queued: Option<Mono>,
    /// When a frame was first queued with none landing since.
    unanswered_since: Option<Mono>,
}

impl FlipWatch {
    /// A frame was queued at `now`.
    #[must_use]
    pub fn queued(self, now: Mono) -> FlipWatch {
        FlipWatch {
            queued: Some(now),
            unanswered_since: self.unanswered_since.or(Some(now)),
        }
    }

    /// A frame was asked for at `now` and could not be queued (the draw or the commit failed):
    /// no flip is in flight, but the time nothing has landed starts running all the same, so a
    /// surface whose every frame fails is starved like one whose flips never land (CASB-05).
    #[must_use]
    pub fn asked(self, now: Mono) -> FlipWatch {
        FlipWatch {
            unanswered_since: self.unanswered_since.or(Some(now)),
            ..self
        }
    }

    /// A flip landed.
    #[must_use]
    pub fn landed(self) -> FlipWatch {
        FlipWatch::default()
    }

    /// The flip in flight was given up (the surface was reset); the time since a frame last
    /// landed keeps running.
    #[must_use]
    pub fn given_up(self) -> FlipWatch {
        FlipWatch {
            queued: None,
            ..self
        }
    }

    /// The output went dark or the seat came back: nothing is expected of it.
    #[must_use]
    pub fn idle(self) -> FlipWatch {
        FlipWatch::default()
    }

    /// Whether the queued flip was queued longer ago than [`FLIP_DEADLINE`] at `now`. Nothing
    /// queued is never overdue.
    pub fn flip_lost(self, now: Mono) -> bool {
        self.queued.is_some_and(|at| now.since(at) > FLIP_DEADLINE)
    }

    /// Whether the output has shown nothing for too long at `now`: frames were asked for for
    /// [`STARVED_AFTER`] and none landed.
    pub fn starved(self, now: Mono) -> bool {
        self.unanswered_since
            .is_some_and(|at| now.since(at) > STARVED_AFTER)
    }
}

/// Frames that failed one after another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FailureRun(u32);

/// What a failed frame means for the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failing {
    /// Report it and try the next frame.
    Carry,
    /// Too many in a row: set the output up again.
    Reinit,
}

impl FailureRun {
    /// The run after another failed frame, and what to do about it.
    #[must_use]
    pub fn failed(self) -> (FailureRun, Failing) {
        let run = FailureRun(self.0.saturating_add(1));
        match run.0 >= FAILURES_ALLOWED {
            true => (FailureRun::default(), Failing::Reinit),
            false => (run, Failing::Carry),
        }
    }

    /// A frame went through: the run ends.
    #[must_use]
    pub fn succeeded(self) -> FailureRun {
        FailureRun::default()
    }
}

/// How soon after one recovery another one means the first did not help.
pub const RECOVERY_WINDOW: Duration = Duration::from_secs(5);

/// How hard to try to get a frozen output going.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Make the surface forget the screen and draw the whole picture again.
    Reset,
    /// Throw the output's surface away and light it again from the model's configuration.
    Reinit,
}

/// The recoveries of one output, to tell a first try from a second one that follows soon after.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Recoveries {
    last: Option<Mono>,
}

impl Recoveries {
    /// The recoveries after one more at `now`, and how hard to try: a reset, unless another
    /// recovery was needed within [`RECOVERY_WINDOW`], then a re-initialisation (which starts
    /// the count over).
    #[must_use]
    pub fn next(self, now: Mono) -> (Recoveries, Level) {
        match self.last {
            Some(last) if now.since(last) < RECOVERY_WINDOW => {
                (Recoveries::default(), Level::Reinit)
            }
            _ => (Recoveries { last: Some(now) }, Level::Reset),
        }
    }
}

/// Whether the display is showing the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    /// At least one output shows frames, or none is expected to.
    Alive,
    /// Every output that should show frames has shown none for [`STARVED_AFTER`].
    Frozen,
    /// The seat is gone for good: nothing will be shown again.
    SeatLost,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: u64) -> Mono {
        Mono(Duration::from_millis(ms))
    }

    #[test]
    fn a_flip_is_lost_only_after_its_deadline_and_a_landing_clears_it() {
        let watch = FlipWatch::default();
        assert!(!watch.flip_lost(at(10_000)), "nothing queued");
        let watch = watch.queued(at(1_000));
        assert!(!watch.flip_lost(at(1_500)), "exactly at the deadline");
        assert!(watch.flip_lost(at(1_501)));
        assert!(!watch.landed().flip_lost(at(9_000)));
    }

    #[test]
    fn starvation_survives_giving_a_flip_up_but_not_a_landing() {
        let watch = FlipWatch::default().queued(at(0));
        assert!(!watch.starved(at(10_000)));
        assert!(watch.starved(at(10_001)));
        // A recovery drops the flip in flight, not the time nothing has landed.
        let given_up = watch.given_up().queued(at(9_000));
        assert!(given_up.starved(at(10_001)));
        assert!(!watch.landed().starved(at(60_000)));
        assert!(!watch.idle().starved(at(60_000)));
    }

    /// CASB-05: frames that fail to queue count as asked for. One failure starts the clock; it
    /// is not restarted by the next, and a landing clears it.
    #[test]
    fn a_surface_whose_every_frame_fails_to_queue_is_starved_like_one_that_never_flips() {
        let watch = FlipWatch::default().asked(at(1_000));
        assert!(!watch.flip_lost(at(60_000)), "no flip is in flight");
        assert!(!watch.starved(at(11_000)));
        assert!(watch.starved(at(11_001)));
        let again = watch.asked(at(9_000));
        assert!(again.starved(at(11_001)), "the clock keeps its start");
        assert!(!again.landed().starved(at(60_000)));
    }

    #[test]
    fn the_third_failure_in_a_row_asks_for_a_new_start_and_a_success_resets_the_count() {
        let run = FailureRun::default();
        let (run, first) = run.failed();
        let (run, second) = run.failed();
        assert_eq!((first, second), (Failing::Carry, Failing::Carry));
        let (again, third) = run.failed();
        assert_eq!(third, Failing::Reinit);
        assert_eq!(again, FailureRun::default());
        let (after_success, next) = run.succeeded().failed();
        assert_eq!(next, Failing::Carry);
        assert_ne!(after_success, FailureRun::default());
    }

    #[test]
    fn a_second_recovery_soon_after_the_first_is_a_harder_one() {
        let (recoveries, first) = Recoveries::default().next(at(1_000));
        assert_eq!(first, Level::Reset);
        let (recoveries, second) = recoveries.next(at(2_000));
        assert_eq!(second, Level::Reinit);
        let (recoveries, third) = recoveries.next(at(3_000));
        assert_eq!(
            third,
            Level::Reset,
            "the count starts over after a re-initialisation"
        );
        let (_, late) = recoveries.next(at(3_000 + 5_000));
        assert_eq!(
            late,
            Level::Reset,
            "a recovery long after the last is a first again"
        );
    }
}
