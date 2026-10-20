//! What an agent lease's input may do before it is delivered at all (`Origin::Agent`). The
//! reach table keeps it to the leased window by what is under the pointer and where the model
//! sends keys; this gate adds what the seat and the compositor's own interactions know:
//!
//! - it never drives the compositor: no Mission Control, no window frame, no Split View
//!   divider, nothing while a move, resize or divider drag is in progress;
//! - it never reaches a client through another client's grab: while the pointer or the keyboard
//!   is held by a surface that is not the leased window's client, it is dropped;
//! - only keys, pointer motion, buttons and scrolling: no touch, gestures or switches.
//!
//! The release of a key or button is never stopped here (nor by the reach table, `deliver`):
//! the lease's socket drops a release of what it does not hold, and a press that was refused
//! takes its release with it, so what reaches the seat is the end of a press the client was
//! given, and a key or button is never left down.

use super::{divider, frames, overview};
use crate::state::State;
use casement_core::geometry::LogicalPoint;
use casement_core::input::{Edge, InputEvent, InputKind};
use casement_core::{ClientKey, LeaseId};
use casement_wayland::ClientState;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

/// Whether an agent event goes on to delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Gate {
    /// It goes on (the reach table still has its say).
    Open,
    /// It is dropped.
    Shut,
}

/// The gate for `event` of `lease`, with the pointer at `at`.
pub(super) fn gate(
    state: &State,
    lease: LeaseId,
    event: &InputEvent,
    at: Option<LogicalPoint>,
) -> Gate {
    if is_release(&event.kind) {
        return Gate::Open;
    }
    let Some(client) = leased_client(state, lease) else {
        return Gate::Shut;
    };
    let shut = !allowed_kind(&event.kind)
        || overview::takes_input(state)
        || state.wl.seat.grab.is_some()
        || divider::is_dragging(state)
        || held_by_another(state, client, &event.kind)
        || on_compositor_chrome(state, &event.kind, at);
    match shut {
        true => Gate::Shut,
        false => Gate::Open,
    }
}

fn is_release(kind: &InputKind) -> bool {
    matches!(
        kind,
        InputKind::Key {
            edge: Edge::Release,
            ..
        } | InputKind::Button {
            edge: Edge::Release,
            ..
        }
    )
}

fn allowed_kind(kind: &InputKind) -> bool {
    matches!(
        kind,
        InputKind::Key { .. }
            | InputKind::Motion { .. }
            | InputKind::MotionTo { .. }
            | InputKind::Button { .. }
            | InputKind::Axis(_)
    )
}

/// The client of the window `lease` holds; none for a lease that ended.
fn leased_client(state: &State, lease: LeaseId) -> Option<ClientKey> {
    let model = &state.core.model;
    let window = model.leases().window_of(lease)?;
    model.windows().get(&window).map(|held| held.client)
}

fn client_of(surface: &WlSurface) -> Option<ClientKey> {
    surface
        .client()
        .and_then(|client| ClientState::of(&client).map(|data| data.key))
}

/// Whether the pointer (for pointer events) or the keyboard (for keys) is grabbed by, or
/// focused on, a surface of another client than `leased`: a popup grab, a drag, an input
/// method's or another app's implicit grab would carry the event there.
fn held_by_another(state: &State, leased: ClientKey, kind: &InputKind) -> bool {
    let seat = &state.wl.seat;
    let elsewhere = |surface: Option<WlSurface>| {
        surface
            .as_ref()
            .and_then(client_of)
            .is_some_and(|client| client != leased)
    };
    match kind {
        InputKind::Key { .. } => elsewhere(seat.keyboard.current_focus()),
        _ => {
            seat.pointer.is_grabbed()
                && elsewhere(seat.pointer.current_focus().and_then(Into::into))
        }
    }
}

/// Whether a press lands on what the compositor draws and acts on itself: a window's frame
/// (its buttons, its title bar, its edges) or a Split View divider.
fn on_compositor_chrome(state: &State, kind: &InputKind, at: Option<LogicalPoint>) -> bool {
    let (InputKind::Button { .. }, Some(at)) = (kind, at) else {
        return false;
    };
    let stacked = super::scene::stack(state);
    frames::pick(state, at).0.is_some() || divider::under_pointer(state, &stacked, at).is_some()
}
