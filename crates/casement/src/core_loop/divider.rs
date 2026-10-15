//! The Split View divider in the loop. The strip between the halves is the compositor's, as on
//! the Mac: the pointer over it shows the left-right resize cursor, a press on it starts a drag,
//! and each motion until the button comes up sets the leading window's share
//! (`ShellRequest::SplitDivide`), which the model keeps for the life of the Space and layout
//! turns into the halves. No client hears the pointer while it is on the strip or dragging.

use super::feed;
use super::grab::Follow;
use super::scene::{Hit, hit_test};
use crate::state::State;
use casement_core::geometry::LogicalPoint;
use casement_core::input::ButtonCode;
use casement_core::layer::LayerLevel;
use casement_core::lock::LockState;
use casement_core::shell::ShellRequest;
use casement_core::{Event, Model, OutputKey};
use casement_layout::DividerTarget;
use casement_render::{CursorDraw, CursorShape, Rgba8};
use casement_wayland::seat::Candidate;

/// What fills the output behind the halves and in the gap between them (a dark grey, as the
/// Mac's Split View has).
const BACKDROP: Rgba8 = Rgba8 {
    r: 28,
    g: 28,
    b: 30,
    a: 255,
};

/// Where the pointer is in relation to a Split View divider.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum DividerPointer {
    /// Not on a divider.
    #[default]
    Away,
    /// On the strip, no button down: the resize cursor shows.
    Over,
    /// A button went down on the strip and has not come up.
    Dragging {
        /// The divider being dragged, as layout placed it when the press came.
        divider: DividerTarget,
        /// The button that began it; its release ends the drag.
        button: ButtonCode,
    },
}

/// The pointer moved to `at`. While dragging, the share follows it; otherwise the cursor is the
/// resize cursor over a strip and the arrow again after leaving it. `Taken` when no client
/// should hear this motion.
pub(super) fn moved(state: &mut State, stacked: &[(Hit, Candidate)], at: LogicalPoint) -> Follow {
    if let DividerPointer::Dragging { divider, .. } = &state.render.divider {
        let request = ShellRequest::SplitDivide {
            window: divider.leading,
            share: divider.share_at(at.x),
        };
        feed(state, vec![Event::Shell(request)]);
        return Follow::Taken;
    }
    let was = std::mem::take(&mut state.render.divider);
    match (under_pointer(state, stacked, at), was) {
        (Some(_), _) => {
            state.render.divider = DividerPointer::Over;
            state.render.chrome.cursor = CursorDraw::Named(CursorShape::ResizeEw);
            Follow::Taken
        }
        (None, DividerPointer::Over) => {
            state.render.chrome.cursor = CursorDraw::Named(CursorShape::Default);
            Follow::Free
        }
        (None, _) => Follow::Free,
    }
}

/// The drag of `divider` begins with `button`.
pub(super) fn begin(state: &mut State, divider: DividerTarget, button: ButtonCode) {
    state.render.divider = DividerPointer::Dragging { divider, button };
    state.render.chrome.cursor = CursorDraw::Named(CursorShape::ResizeEw);
}

/// Whether releasing `released` ends the drag in progress.
pub(super) fn ends_with(state: &State, released: ButtonCode) -> bool {
    matches!(&state.render.divider, DividerPointer::Dragging { button, .. } if *button == released)
}

/// `released` came up: the drag, if it began with that button, ends (the pointer is still on the
/// strip, so it stays `Over`).
pub(super) fn end(state: &mut State, released: ButtonCode) {
    if ends_with(state, released) {
        state.render.divider = DividerPointer::Over;
    }
}

/// Whether a drag holds the pointer.
pub(super) fn is_dragging(state: &State) -> bool {
    matches!(state.render.divider, DividerPointer::Dragging { .. })
}

/// The backdrop to draw on `output`: its shown Space is a Split View.
pub(super) fn backdrop(state: &State, output: &OutputKey) -> Option<Rgba8> {
    let model = &state.core.model;
    let shown = model.spaces().shown_on(output)?;
    state
        .core
        .arrangement
        .backdrops
        .iter()
        .any(|target| target.output == *output && target.space == shown.id)
        .then_some(BACKDROP)
}

/// The divider of a shown Split View whose strip is under `at`, unless something in front of it
/// (the lock, an overlay layer, a window's own surface) takes the pointer there.
pub(super) fn under_pointer(
    state: &State,
    stacked: &[(Hit, Candidate)],
    at: LogicalPoint,
) -> Option<DividerTarget> {
    let model = &state.core.model;
    let found = state
        .core
        .arrangement
        .dividers
        .iter()
        .filter(|divider| divider.hit(at))
        .find(|divider| shown(model, divider))?;
    let unlocked = matches!(model.lock(), LockState::Unlocked);
    let front = hit_test(stacked, at).map(|(hit, _)| hit);
    (unlocked && !in_front(model, front.as_ref())).then(|| found.clone())
}

/// Whether the surface found at a point takes the pointer before a divider would. Layers below
/// the full-screen windows do not: they are covered there.
fn in_front(model: &Model, front: Option<&Hit>) -> bool {
    match front {
        None => false,
        Some(Hit::Lock(_) | Hit::Window(_)) => true,
        Some(Hit::Layer(key)) => model
            .layers()
            .get(key)
            .is_some_and(|spec| spec.level == LayerLevel::Overlay),
    }
}

/// Whether the divider's Space is the one its output shows.
fn shown(model: &Model, divider: &DividerTarget) -> bool {
    model
        .spaces()
        .shown_on(&divider.output)
        .is_some_and(|space| space.id == divider.space)
}
