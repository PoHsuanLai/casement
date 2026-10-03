//! Retargeting: from one arrangement to the next, every leg starting from where it is at the
//! speed it has (design/27 §3.12 rule 4).

use crate::book::{Motion, MotionBook, MotionKey};
use crate::cause_travel::{Travel, travel_of};
use crate::goals::{Goal, Goals};
use crate::pose::Pose;
use crate::space_slide::SlideLeg;
use crate::window_motion::{Fate, WindowLegs};
use casement_core::Cause;
use casement_core::WindowId;
use casement_core::time::Mono;
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
    let travel = travel_of(cause, level);
    let (before, after) = (Goals::of(from), Goals::of(to));
    let mut tracked = book.legs;
    tracked.retain(|_, motion| !motion.is_gone(now));
    let ids: BTreeSet<WindowId> = tracked
        .keys()
        .filter_map(|key| match key {
            MotionKey::Window(id) => Some(*id),
            _ => None,
        })
        .chain(before.windows.keys().copied())
        .chain(after.windows.keys().copied())
        .collect();
    let step = Step {
        cause,
        travel,
        level,
        now,
    };
    let mut legs = BTreeMap::new();
    for id in ids {
        let known = match tracked.remove(&MotionKey::Window(id)) {
            Some(Motion::Window(legs)) => Some(legs),
            _ => None,
        };
        let was = before.windows.get(&id);
        let next = match after.windows.get(&id) {
            Some(goal) => Some(step.toward(known, was, goal)),
            None => step.away(known, was),
        };
        legs.extend(next.map(|next| (MotionKey::Window(id), Motion::Window(next))));
    }
    for (output, target) in &after.strips {
        let key = MotionKey::SpaceStrip(output.clone());
        let known = match tracked.remove(&key) {
            Some(Motion::Slide(leg)) => leg,
            _ => SlideLeg::still(before.strips.get(output).copied().unwrap_or(*target), now),
        };
        let thrown = travel
            .spring()
            .map_or(0.0, |spec| spec.thrown(level).px_per_s());
        let aimed = known.aimed(*target, thrown, travel, level, now);
        legs.insert(key, Motion::Slide(aimed));
    }
    MotionBook { legs }
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
        match known.or_else(|| was.map(|was| self.standing(was))) {
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
        }
    }

    /// The legs for a window that is not in the new arrangement: it fades out (unless it
    /// already is). A window nothing tracks and the old arrangement did not hold is nothing.
    fn away(&self, known: Option<WindowLegs>, was: Option<&Goal>) -> Option<WindowLegs> {
        let legs = known.or_else(|| was.map(|was| self.standing(was)))?;
        match legs.fate {
            Fate::Leaving => Some(legs),
            Fate::Present => {
                let exit = self.leaving_to(legs.pose(self.now));
                Some(
                    legs.with_fate(Fate::Leaving)
                        .aimed(exit, self.travel, self.level, self.now),
                )
            }
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
