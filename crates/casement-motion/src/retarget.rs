//! Retargeting: from one arrangement to the next, every leg starting from where it is at the
//! speed it has (design/27 §3.12 rule 4).

use crate::book::MotionBook;
use crate::cause_travel::{Travel, travel_of};
use crate::flight::FlightEnds;
use crate::goals::{Goal, Goals};
use crate::pose::Pose;
use crate::space_slide::SlideLeg;
use crate::window_motion::{Fate, WindowLegs};
use casement_core::Cause;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::time::Mono;
use casement_core::{OutputKey, WindowId};
use casement_layout::Arrangement;
use ds_style::appearance::motion::MotionLevel;
use std::collections::{BTreeMap, BTreeSet};

/// The book after the arrangement changed from `from` to `to` at `now` because of `cause`.
/// A leg whose target changed starts a new leg from its state at `now` (position and velocity
/// kept); a window that appears fades and grows in from its target, one that leaves fades out
/// (a minimising one toward its dock tile); the spring comes from `cause`
/// (`SpringSpec::for_touch`), and under [`MotionLevel::Reduced`] nothing travels: positions and
/// sizes are at their targets at once and only arrivals and departures cross-fade.
pub fn retarget(
    book: MotionBook,
    from: &Arrangement,
    to: &Arrangement,
    cause: &Cause,
    level: MotionLevel,
    now: Mono,
) -> MotionBook {
    retarget_driven(book, from, to, cause, level, now, &BTreeSet::new())
}

/// [`retarget`] while a swipe drives the Space strips of the outputs in `driven`: the renderer
/// draws those rows at the swipe's own position (its rubber band and its ease-out finish), so
/// their strip legs go to the new target at once instead of sliding as well, and a commit does
/// not slide twice. Windows and the other outputs' strips travel as `retarget` has them.
pub fn retarget_driven(
    book: MotionBook,
    from: &Arrangement,
    to: &Arrangement,
    cause: &Cause,
    level: MotionLevel,
    now: Mono,
    driven: &BTreeSet<OutputKey>,
) -> MotionBook {
    let travel = travel_of(cause, level);
    let (before, after) = (Goals::of(from), Goals::of(to));
    let MotionBook {
        windows: mut tracked,
        strips: mut tracked_strips,
    } = book;
    tracked.retain(|_, legs| !legs.is_gone(now));
    let ids: BTreeSet<WindowId> = tracked
        .keys()
        .copied()
        .chain(before.windows.keys().copied())
        .chain(after.windows.keys().copied())
        .collect();
    let step = Step {
        cause,
        travel,
        level,
        now,
    };
    let mut windows = BTreeMap::new();
    for id in ids {
        let known = tracked.remove(&id);
        let was = before.windows.get(&id);
        let next = match after.windows.get(&id) {
            Some(goal) => Some(step.toward(known, was, goal)),
            None => step.away(known, was),
        };
        windows.extend(next.map(|next| (id, next)));
    }
    let mut strips = BTreeMap::new();
    for (output, target) in &after.strips {
        let known = tracked_strips.remove(output).unwrap_or_else(|| {
            SlideLeg::still(before.strips.get(output).copied().unwrap_or(*target), now)
        });
        let slide = match driven.contains(output) {
            true => Travel::Snap,
            false => travel,
        };
        let thrown = slide
            .spring()
            .map_or(0.0, |spec| spec.thrown(level).px_per_s());
        let aimed = known.aimed(*target, thrown, slide, level, now);
        strips.insert(output.clone(), aimed);
    }
    MotionBook { windows, strips }
}

/// The parts of a retarget every window shares.
struct Step<'a> {
    cause: &'a Cause,
    travel: Travel,
    level: MotionLevel,
    now: Mono,
}

impl Step<'_> {
    /// The legs for a window that is in the new arrangement: the ones it has, else the ones it
    /// had standing, else an arrival.
    fn toward(&self, known: Option<WindowLegs>, was: Option<&Goal>, goal: &Goal) -> WindowLegs {
        let shown = Pose::shown(goal.rect);
        // Coming back is a window that was leaving or that nothing knew: a flight out of the tile.
        let returning = match (&known, was) {
            (Some(legs), _) => legs.fate == Fate::Leaving,
            (None, was) => was.is_none(),
        };
        let flight = match returning {
            true => self.flight(goal.rect),
            false => None,
        };
        let legs = match known.or_else(|| was.map(|was| self.standing(was))) {
            Some(legs) => legs
                .with_fate(Fate::Present)
                .placed(goal.output.clone(), goal.stack)
                .aimed(shown, self.travel, self.level, self.now),
            None => WindowLegs::still(
                self.arriving_from(goal),
                goal.output.clone(),
                goal.stack,
                self.now,
            )
            .aimed(shown, self.travel, self.level, self.now),
        };
        legs.with_flight(flight)
    }

    /// The legs for a window that is not in the new arrangement: it fades out (unless it
    /// already is). A window nothing tracks and the old arrangement did not hold is nothing.
    fn away(&self, known: Option<WindowLegs>, was: Option<&Goal>) -> Option<WindowLegs> {
        let legs = known.or_else(|| was.map(|was| self.standing(was)))?;
        match legs.fate {
            Fate::Leaving => Some(legs),
            Fate::Present => {
                let here = legs.pose(self.now);
                let exit = self.leaving_to(here);
                let flight = self.flight(rect_of(here));
                Some(
                    legs.with_fate(Fate::Leaving)
                        .aimed(exit, self.travel, self.level, self.now)
                        .with_flight(flight),
                )
            }
        }
    }

    /// The ends of a flight between `window` and the dock tile, when this retarget is a minimise
    /// that travels (nothing flies under reduced motion).
    fn flight(&self, window: LogicalRect) -> Option<FlightEnds> {
        match (self.travel.spring(), self.cause) {
            (Some(_), Cause::Minimise(target)) => Some(FlightEnds {
                window,
                tile: target.rect,
            }),
            _ => None,
        }
    }

    /// A window the book did not track, standing where the old arrangement had it.
    fn standing(&self, was: &Goal) -> WindowLegs {
        WindowLegs::still(
            Pose::shown(was.rect),
            was.output.clone(),
            was.stack,
            self.now,
        )
    }

    /// Where an arriving window starts: invisible at its target, a little smaller (or at its
    /// dock tile when it comes out of one); at the target itself when nothing travels.
    fn arriving_from(&self, goal: &Goal) -> Pose {
        let at_goal = Pose::shown(goal.rect).faded();
        match (self.travel.spring(), self.cause) {
            (None, _) => at_goal,
            (Some(_), Cause::Minimise(target)) => Pose::shown(target.rect).faded(),
            (Some(_), _) => at_goal.tucked(),
        }
    }

    /// Where a leaving window ends: invisible, a little smaller (or at its dock tile when it
    /// goes into one); where it is when nothing travels.
    fn leaving_to(&self, here: Pose) -> Pose {
        match (self.travel.spring(), self.cause) {
            (None, _) => here.faded(),
            (Some(_), Cause::Minimise(target)) => Pose::shown(target.rect).faded(),
            (Some(_), _) => here.tucked().faded(),
        }
    }
}

/// `pose` as whole logical pixels.
fn rect_of(pose: Pose) -> LogicalRect {
    let whole = |value: f64| value.round() as i32;
    let side = |value: f64| value.round().max(0.0) as u32;
    LogicalRect::new(
        LogicalPoint {
            x: whole(pose.x),
            y: whole(pose.y),
        },
        LogicalSize {
            w: side(pose.w),
            h: side(pose.h),
        },
    )
}
