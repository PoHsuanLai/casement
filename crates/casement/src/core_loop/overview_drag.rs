//! Dragging a window to a Space in Mission Control, as the loop carries it: a press on a slot
//! starts a [`Drag`]; past the threshold the window follows the pointer (its slot moves, motion
//! carries the picture), the shell hears which Space target is under it (`drop_target`), and the
//! release drops it on that Space, flies it back to its slot, or, if the pointer never moved
//! far, picks it as a press always did. The machine and the targets are `casement-core`'s
//! (`overview_drag`); this is the glue to the seat, motion and the model.

use super::overview::{escape_leaves, leave};
use crate::exec;
use crate::state::State;
use casement_core::desktop::{DesktopRequest, WindowAsk};
use casement_core::geometry::LogicalPoint;
use casement_core::input::InputEvent;
use casement_core::overview_drag::{Drag, Moved, Over, Released};
use casement_core::shell::ShellNote;
use casement_core::{Cause, Effect, Event, OutputKey, OverviewPick, WindowId};
use casement_motion::retarget;
use casement_render::{CursorDraw, CursorShape};

/// Whether the drag took the pointer move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Follow {
    /// A window is being dragged: it followed and the cursor is the closed hand.
    Taken,
    /// No drag: the pointer only hovers.
    Free,
}

/// A press on `window`'s slot at `at`: it may become a drag, or be a pick.
pub(super) fn press(state: &mut State, event: &InputEvent, window: WindowId, at: LogicalPoint) {
    let slot = state
        .core
        .arrangement
        .overview
        .iter()
        .find(|slot| slot.window == window)
        .map_or(at, |slot| slot.rect.origin);
    state.render.overview_drag = Drag::press(window, event.origin, at, slot);
}

/// The pointer moved to `at`: a drag in progress follows it.
pub(super) fn moved(state: &mut State, at: LogicalPoint) -> Follow {
    if state.render.overview_drag == Drag::Idle {
        return Follow::Free;
    }
    let drag = std::mem::take(&mut state.render.overview_drag);
    let before = drag.over().cloned();
    let (drag, step) = {
        let model = &state.core.model;
        drag.moved(at, &state.wl.quire_shell.space_targets, |output| {
            model.outputs().0.contains_key(output)
        })
    };
    state.render.overview_drag = drag;
    let Moved::Follows {
        window,
        top_left,
        changed,
    } = step
    else {
        return Follow::Free;
    };
    follow(state, window, top_left);
    state.render.overview_hover = Some(window);
    state.render.chrome.cursor = CursorDraw::Named(CursorShape::Grabbing);
    if let Some(now) = changed {
        tell(state, before.as_ref(), now.as_ref());
    }
    Follow::Taken
}

/// The button went up: the press ends as a pick, a drop on a Space, or a flight back.
pub(super) fn released(state: &mut State) {
    let drag = std::mem::take(&mut state.render.overview_drag);
    let over = drag.over().cloned();
    let ended = drag.released();
    if !escape_leaves(state) {
        return;
    }
    match ended {
        Released::Nothing => {}
        Released::Pick { window, origin } => {
            leave(state, Some(OverviewPick { window, origin }));
        }
        Released::Drop {
            window,
            over: target,
        } => {
            tell(state, Some(&target), None);
            state.render.overview_hover = None;
            let ask = WindowAsk::ToSpace(target.space);
            super::feed(
                state,
                vec![Event::Desktop(DesktopRequest::Window { window, ask })],
            );
        }
        Released::FlyBack { .. } => {
            tell(state, over.as_ref(), None);
            state.render.overview_hover = None;
            super::relayout(state, &Cause::Overview);
        }
        _ => {}
    }
}

/// The dragged `window`'s slot is `top_left` now (and the front-most): motion carries it there.
fn follow(state: &mut State, window: WindowId, top_left: LogicalPoint) {
    let now = state.clock.now();
    let mut next = state.core.arrangement.clone();
    let Some(at) = next.overview.iter().position(|slot| slot.window == window) else {
        return;
    };
    let mut slot = next.overview.remove(at);
    slot.rect.origin = top_left;
    next.overview.push(slot);
    let motion = std::mem::replace(&mut state.core.motion, casement_motion::MotionBook::empty());
    state.core.motion = retarget(
        motion,
        &state.core.arrangement,
        &next,
        &Cause::Drag,
        Default::default(),
        now,
    );
    state.core.arrangement = next;
}

/// The shell hears the Space target under the dragged window changed from `before` to `now`.
fn tell(state: &mut State, before: Option<&Over>, now: Option<&Over>) {
    let mut notes: Vec<Effect> = Vec::new();
    let cleared: Option<&OutputKey> = before
        .map(|over| &over.output)
        .filter(|output| now.is_none_or(|over| over.output != **output));
    if let Some(output) = cleared {
        notes.push(Effect::Shell(ShellNote::DropTarget {
            output: output.clone(),
            space: None,
        }));
    }
    if let Some(over) = now {
        notes.push(Effect::Shell(ShellNote::DropTarget {
            output: over.output.clone(),
            space: Some(over.space.clone()),
        }));
    }
    exec::exec(state, notes);
}
