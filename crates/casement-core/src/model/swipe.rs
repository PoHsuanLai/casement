//! The multi-finger swipes in the model (03 section 4.2): the horizontal one moves the row of
//! Spaces under the fingers, the vertical one opens and closes Mission Control.
//!
//! `input::gesture_route` says whose each swipe step is and which axis it drives. The Space
//! swipe is ds-behaviour's machine (`space_swipe::Swipe`), stepped here at the model's time: its
//! offsets become the [`SpaceStrip`] the renderer draws, its commit shows the Space it chose at
//! the lift (focus and windows follow at once, as on the Mac) while the strip finishes sliding,
//! and a timer at the finish's end settles it. The vertical swipe is `input::lift`: its
//! progress is [`Model::overview_progress`], and its verdict opens or closes Mission Control
//! through the same paths a key does.
//!
//! The clients get the swipes the desktop does not take; [`Model::swipe_owner`] says, right
//! after a swipe step was stepped, whose that step was.

use super::Model;
use super::moved::Moved;
use crate::effect::Effect;
use crate::geometry::{LogicalRect, SubPx};
use crate::ids::OutputKey;
use crate::input::gesture_route::{Axis, Drive, Fate, Route, route};
use crate::input::lift::{Intent, Lift, LiftEnd, LiftIn, LiftOut};
use crate::input::{InputEvent, InputKind, Origin};
use crate::lock::LockState;
use crate::overview::Overview;
use crate::time::{Due, Mono, TimerKey};
use ds_behaviour::space_swipe::{
    PageMilli, Reduced, Swipe, SwipeIn, SwipeOut, SwipeParams, SwipeSource, finish_duration,
};
use ds_core::machine::Machine;
use std::time::Duration;

/// Whose a swipe step is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwipeOwner {
    /// The clients': deliver it to the surface under the pointer.
    #[default]
    Client,
    /// The desktop's: it was taken, so no client sees it.
    Desktop,
}

/// Where the row of Spaces is drawn while a swipe moves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceStrip {
    /// The output whose Spaces move.
    pub output: OutputKey,
    /// The row's position in thousandths of a page, growing toward later Spaces: Space `k` at
    /// `k * 1000`; the rubber band at the ends is already applied.
    pub position: PageMilli,
}

impl SpaceStrip {
    /// Where the row's first Space starts, in global sub-pixels, when each Space is as wide as
    /// `usable` (the output's usable area): the shown Space sits at `usable`'s left edge when the
    /// position is its index, and a neighbour is one page further per thousand.
    pub fn first_x(&self, usable: LogicalRect) -> SubPx {
        let width = i32::try_from(usable.size.w).unwrap_or(i32::MAX);
        let page = i64::from(SubPx::of_logical(width).0);
        let shift = i64::from(self.position.0) * page / 1000;
        SubPx(clamp(
            i64::from(SubPx::of_logical(usable.origin.x).0) - shift,
        ))
    }
}

/// Everything the swipes keep between steps.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct SwipeState {
    route: Route,
    /// The Space swipe, at rest on the first Space until it first begins.
    space: Swipe,
    lift: Lift,
    /// The output whose row the Space swipe moves.
    output: Option<OutputKey>,
    /// Where the fingers have put the row (while they are down).
    following: Option<PageMilli>,
    /// How open the fingers have made Mission Control (while they are down).
    progress: Option<PageMilli>,
    owner: SwipeOwner,
}

impl Model {
    /// Whose the latest swipe step was: ask right after the step went through `step`.
    pub fn swipe_owner(&self) -> SwipeOwner {
        self.swipe.owner
    }

    /// Where the row of Spaces is drawn at `now` while a swipe or its finish moves it; none
    /// when the Spaces sit still.
    pub fn space_strip(&self, now: Mono) -> Option<SpaceStrip> {
        let output = self.swipe.output.clone()?;
        let position = match &self.swipe.space {
            Swipe::Idle { .. } => return None,
            Swipe::Tracking { .. } => self.swipe.following?,
            Swipe::Finishing {
                to, p0, v0, since, ..
            } => {
                let to = PageMilli::of_space(*to);
                let delta = i64::from(to.0) - i64::from(p0.0);
                let total = finish_duration(PageMilli(clamp(delta)), *v0);
                let elapsed = Duration::from_millis(now.stamp().since(*since));
                let u = elapsed.as_millis() * 1000 / total.as_millis().max(1);
                let rest = 1000 - i64::try_from(u.min(1000)).unwrap_or(1000);
                let eased = 1_000_000_000 - rest * rest * rest;
                PageMilli(clamp(i64::from(p0.0) + delta * eased / 1_000_000_000))
            }
        };
        Some(SpaceStrip { output, position })
    }

    /// How open the fingers have made Mission Control, in thousandths (0 closed, 1000 open),
    /// while a vertical swipe is down; none otherwise.
    pub fn overview_progress(&self) -> Option<PageMilli> {
        self.swipe.progress
    }

    /// A swipe step: whose it is, and what it drives.
    pub(super) fn swipe_input(self, event: InputEvent) -> Moved {
        let InputKind::Swipe(step) = event.kind else {
            return Moved::of(self);
        };
        let mut model = self;
        let ours = matches!(model.lock, LockState::Unlocked)
            && (event.origin.is_physical()
                || event.origin.is_pad()
                || matches!(event.origin, Origin::Feed(_)));
        let (next, fate) = match ours {
            true => route(model.swipe.route, step, &model.settings.gestures),
            false => abandoned(model.swipe.route),
        };
        model.swipe.route = next;
        let drives = match fate {
            Fate::Pass => {
                model.swipe.owner = SwipeOwner::Client;
                Vec::new()
            }
            Fate::Take(drives) => {
                model.swipe.owner = SwipeOwner::Desktop;
                drives
            }
        };
        drives.into_iter().fold(Moved::of(model), |moved, drive| {
            moved.then(|m| m.drive(drive))
        })
    }

    fn drive(self, drive: Drive) -> Moved {
        match drive {
            Drive::Began(Axis::Horizontal) => self.space_swipe_began(),
            Drive::Moved(Axis::Horizontal, dx) => self.space_swipe(SwipeIn::Changed { dx }),
            Drive::Ended(Axis::Horizontal) => self.space_swipe(SwipeIn::Ended),
            Drive::Cancelled(Axis::Horizontal) => self.space_swipe(SwipeIn::Cancelled),
            Drive::Began(Axis::Vertical) => self.lift_began(),
            Drive::Moved(Axis::Vertical, dy) => self.lift_step(LiftIn::Moved { dy }),
            Drive::Ended(Axis::Vertical) => self.lift_step(LiftIn::Ended),
            Drive::Cancelled(Axis::Vertical) => self.lift_step(LiftIn::Cancelled),
        }
    }

    /// The Spaces of `output`, how many.
    fn row_len(&self, output: &OutputKey) -> u32 {
        u32::try_from(self.spaces.row(output).len()).unwrap_or(u32::MAX)
    }

    /// The index in `output`'s row of the Space it shows.
    pub fn shown_index(&self, output: &OutputKey) -> Option<u32> {
        let shown = self.spaces.shown_on(output)?;
        let at = self
            .spaces
            .row(output)
            .iter()
            .position(|space| space.id == shown.id)?;
        u32::try_from(at).ok()
    }

    /// The fingers locked horizontal: they take the row of the Spaces of the output the person
    /// works on, catching a finish still in flight. Mission Control up: nothing to swipe.
    fn space_swipe_began(self) -> Moved {
        let Some(output) = self.working_output() else {
            return Moved::of(self);
        };
        let Some(shown) = self.shown_index(&output) else {
            return Moved::of(self);
        };
        if !matches!(self.overview, Overview::Closed) {
            return Moved::of(self);
        }
        let mut model = self;
        let flying = matches!(
            model.swipe.space,
            Swipe::Finishing { to, .. } if to == shown
        ) && model.swipe.output.as_ref() == Some(&output);
        if !flying {
            model.swipe.space = Swipe::Idle { at: shown };
        }
        model.swipe.output = Some(output.clone());
        let source = SwipeSource::Touchpad;
        Moved::of(model)
            .then(|m| m.space_swipe(SwipeIn::Began { source }))
            .with(Effect::CancelTimer(TimerKey::SpaceFinish(output)))
    }

    /// `input` to the Space swipe at the model's time; what it asks is carried out, and the
    /// timer for the finish's end is armed.
    pub(super) fn space_swipe(self, input: SwipeIn) -> Moved {
        let Some(output) = self.swipe.output.clone() else {
            return Moved::of(self);
        };
        if !self.outputs.0.contains_key(&output) {
            let mut model = self;
            model.swipe = SwipeState::default();
            return Moved::of(model);
        }
        let params = SwipeParams {
            spaces: self.row_len(&output),
            reduced: Reduced::No,
        };
        let (machine, outs) = self
            .swipe
            .space
            .clone()
            .step(input, self.now.stamp(), &params, &());
        let mut model = self;
        model.swipe.space = machine;
        outs.into_iter()
            .fold(Moved::of(model), |moved, out| {
                let output = output.clone();
                moved.then(move |m| m.swiped(out, output))
            })
            .then(|m| m.finish_timer(output))
    }

    /// What the Space swipe asked for.
    fn swiped(self, out: SwipeOut, output: OutputKey) -> Moved {
        match out {
            SwipeOut::Offset { p } => {
                let mut model = self;
                model.swipe.following = Some(p);
                Moved::of(model).with(Effect::RequestFrames(output))
            }
            SwipeOut::Committed { to } => {
                let target = usize::try_from(to).ok().and_then(|at| {
                    self.spaces
                        .row(&output)
                        .get(at)
                        .map(|space| space.id.clone())
                });
                match target {
                    Some(id) => self.show(&id).with(Effect::RequestFrames(output)),
                    None => Moved::of(self),
                }
            }
            SwipeOut::Settled { .. } => {
                let mut model = self;
                model.swipe.following = None;
                Moved::of(model).with(Effect::RequestFrames(output))
            }
        }
    }

    /// The timer that wakes the Space swipe at its finish's end, when it has one.
    fn finish_timer(self, output: OutputKey) -> Moved {
        match self.swipe.space.wake() {
            Some(wake) => {
                let wait = wake.since(self.now.stamp());
                let due = Due {
                    at: self.now.after(Duration::from_millis(wait)),
                    key: TimerKey::SpaceFinish(output),
                };
                Moved::of(self).with(Effect::ArmTimer(due))
            }
            None => Moved::of(self),
        }
    }

    /// The fingers locked vertical: they open Mission Control when it is closed, close it when
    /// it is up, and do nothing while it flies.
    fn lift_began(self) -> Moved {
        let intent = match self.overview {
            Overview::Closed => Some(Intent::Open),
            Overview::Entering | Overview::Open => Some(Intent::Close),
            Overview::Leaving => None,
        };
        match (intent, self.outputs.0.is_empty()) {
            (Some(intent), false) => self.lift_step(LiftIn::Began(intent)),
            _ => Moved::of(self),
        }
    }

    /// `input` to the vertical swipe at the model's time.
    fn lift_step(self, input: LiftIn) -> Moved {
        let (lift, outs) = self.swipe.lift.clone().step(input, self.now.stamp());
        let mut model = self;
        model.swipe.lift = lift;
        outs.into_iter()
            .fold(Moved::of(model), |moved, out| moved.then(|m| m.lifted(out)))
    }

    /// What the vertical swipe asked for.
    fn lifted(self, out: LiftOut) -> Moved {
        let frames: Vec<Effect> = self
            .outputs
            .0
            .keys()
            .cloned()
            .map(Effect::RequestFrames)
            .collect();
        let mut model = self;
        match out {
            LiftOut::Progress(open) => {
                model.swipe.progress = Some(open);
                Moved::of(model).with_all(frames)
            }
            LiftOut::Done(verdict) => {
                // Short of nothing and of everything the windows were drawn part-way.
                let part_way = model
                    .swipe
                    .progress
                    .filter(|open| (1..1000).contains(&open.0));
                model.swipe.progress = None;
                let moved = Moved::of(model).with_all(frames);
                match (verdict, part_way) {
                    (LiftEnd::Open, _) => moved.then(|m| match m.overview {
                        Overview::Closed | Overview::Leaving => m.toggled_overview(),
                        Overview::Entering | Overview::Open => Moved::of(m),
                    }),
                    (LiftEnd::Close, _) => moved.then(|m| m.leave_overview(None)),
                    // The windows were drawn where the fingers put them: they return from there.
                    (LiftEnd::Stay, Some(_)) => moved.then(Model::fingers_stayed),
                    (LiftEnd::Stay, None) => moved,
                }
            }
        }
    }
}

/// A swipe the desktop may no longer take (the session locked, the device went virtual) ends
/// where it is: a swipe it was driving is cancelled, any other is the clients' as it was.
fn abandoned(route: Route) -> (Route, Fate) {
    match route {
        Route::Driving { axis, .. } => (Route::Idle, Fate::Take(vec![Drive::Cancelled(axis)])),
        _ => (Route::Idle, Fate::Pass),
    }
}

/// `value` as an `i32`, held to its range.
fn clamp(value: i64) -> i32 {
    i32::try_from(value.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or_default()
}
