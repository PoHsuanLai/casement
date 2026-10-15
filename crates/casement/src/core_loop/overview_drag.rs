//! Dragging a window to a Space in Mission Control, as the loop carries it: a press on a slot
//! starts a [`Drag`]; past the threshold the window follows the pointer (its slot moves, motion
//! carries the picture), the shell hears the drag begin (`window_drag_begin`) and which Space
//! target is under it (`drop_target`), and the release drops it on that Space, flies it back to its slot, or, if the pointer never moved
//! far, picks it as a press always did. The machine and the targets are `casement-core`'s
//! (`overview_drag`); this is the glue to the seat, motion and the model.

use super::overview::{escape_leaves, leave};
use crate::exec;
use crate::state::State;
use casement_core::desktop::{DesktopRequest, WindowAsk};
use casement_core::geometry::LogicalPoint;
use casement_core::input::InputEvent;
use casement_core::overview_drag::{Drag, Landing, Moved, Over, Released};
use casement_core::shell::ShellNote;
use casement_core::{Cause, Effect, Event, OutputKey, OverviewPick, WindowId};
use casement_motion::retarget;
use casement_protocols::vocab::{DragOutcome, Shown};
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
    let started = !drag.is_dragging();
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
    if started {
        announce(state, |output| ShellNote::WindowDragBegin {
            output,
            window,
        });
    }
    if let Some(now) = changed {
        tell(state, before.as_ref(), now.as_ref());
    }
    Follow::Taken
}

/// The button went up: the press ends as a pick, a drop on a Space, or a flight back.
pub(super) fn released(state: &mut State) {
    let drag = std::mem::take(&mut state.render.overview_drag);
    let over = drag.over().cloned();
    let dragged = drag.window().filter(|_| drag.is_dragging());
    let ended = drag.released();
    if !escape_leaves(state) {
        // Mission Control is already leaving: a drag that began is over, with nothing dropped.
        if let Some(window) = dragged {
            tell(state, over.as_ref(), None);
            finish(state, window, DragOutcome::Cancelled);
        }
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
            let (ask, outcome) = match target.landing {
                Landing::Space(space) => (WindowAsk::ToSpace(space), DragOutcome::DroppedOnSpace),
                // `NewSpace` (the enum grows).
                _ => (
                    WindowAsk::ToNewSpace(target.output),
                    DragOutcome::DroppedOnNew,
                ),
            };
            finish(state, window, outcome);
            land(state, window, ask);
        }
        Released::FlyBack { window, at } => {
            tell(state, over.as_ref(), None);
            // Off every target but over another output: the window goes to the Space that
            // output shows, as a window dragged to another display does on a Mac.
            match shown_space_at(state, window, at) {
                Some(space) => {
                    finish(state, window, DragOutcome::DroppedOnSpace);
                    land(state, window, WindowAsk::ToSpace(space));
                }
                None => {
                    finish(state, window, DragOutcome::Cancelled);
                    state.render.overview_hover = None;
                    super::relayout(state, &Cause::Overview);
                }
            }
        }
        _ => {}
    }
}

/// Mission Control leaves while a window is being dragged: the drag is over, nothing dropped.
pub(super) fn abandon(state: &mut State) {
    let drag = std::mem::take(&mut state.render.overview_drag);
    if let (true, Some(window)) = (drag.is_dragging(), drag.window()) {
        tell(state, drag.over(), None);
        finish(state, window, DragOutcome::Cancelled);
    }
}

/// The shell hears the drag of `window` is over, and how (`window_drag_end`).
fn finish(state: &mut State, window: WindowId, outcome: DragOutcome) {
    announce(state, |output| ShellNote::WindowDragEnd {
        output,
        window,
        outcome,
    });
}

/// One note for every output Mission Control is up on (it is up on all of them at once).
fn announce(state: &mut State, note: impl Fn(OutputKey) -> ShellNote) {
    let notes: Vec<Effect> = state
        .core
        .model
        .outputs()
        .0
        .keys()
        .cloned()
        .map(|output| Effect::Shell(note(output)))
        .collect();
    exec::exec(state, notes);
}

/// Carries out a drop: the model moves `window` (or refuses), and the picture is laid out again
/// either way, so a window the model leaves where it was flies back to its slot.
fn land(state: &mut State, window: WindowId, ask: WindowAsk) {
    state.render.overview_hover = None;
    super::feed(
        state,
        vec![Event::Desktop(DesktopRequest::Window { window, ask })],
    );
    super::relayout(state, &Cause::Overview);
}

/// The Space shown by the output under `at`, when that is not the output `window` is on.
fn shown_space_at(
    state: &State,
    window: WindowId,
    at: LogicalPoint,
) -> Option<porter_core::SpaceId> {
    let model = &state.core.model;
    let under = model
        .outputs()
        .0
        .iter()
        .find(|(_, plugged)| plugged.config.logical_rect().contains(at))
        .map(|(key, _)| key.clone())?;
    let from = model
        .windows()
        .get(&window)
        .and_then(|held| model.output_of(held));
    if from.as_ref() == Some(&under) {
        return None;
    }
    model
        .spaces()
        .shown_on(&under)
        .map(|shown| shown.id.clone())
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

/// The shell hears the target under the dragged window changed from `before` to `now`: the
/// tile it left is cleared (unless the new one is the same kind on the same output, which
/// replaces it) and the one it entered is lit.
fn tell(state: &mut State, before: Option<&Over>, now: Option<&Over>) {
    let mut notes: Vec<Effect> = Vec::new();
    if let Some(left) = before {
        let kept = now.filter(|over| over.output == left.output);
        match (&left.landing, kept.map(|over| &over.landing)) {
            (Landing::Space(_), Some(Landing::Space(_))) => {}
            (Landing::Space(_), _) => notes.push(Effect::Shell(ShellNote::DropTarget {
                output: left.output.clone(),
                space: None,
            })),
            (Landing::NewSpace, Some(Landing::NewSpace)) => {}
            (_, _) => notes.push(Effect::Shell(ShellNote::DropTargetNew {
                output: left.output.clone(),
                over: Shown::Hidden,
            })),
        }
    }
    if let Some(over) = now {
        notes.push(Effect::Shell(match &over.landing {
            Landing::Space(space) => ShellNote::DropTarget {
                output: over.output.clone(),
                space: Some(space.clone()),
            },
            _ => ShellNote::DropTargetNew {
                output: over.output.clone(),
                over: Shown::Shown,
            },
        }));
    }
    exec::exec(state, notes);
}
