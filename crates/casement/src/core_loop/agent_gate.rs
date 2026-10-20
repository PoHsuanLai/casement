//! What an agent lease's input may do before it is delivered at all (`Origin::Agent`). The
//! reach table keeps it to the leased window by what is under the pointer and where the model
//! sends keys; this gate adds what the seat and the compositor's own interactions know:
//!
//! - it never drives the compositor: no Mission Control, no window frame, no Split View
//!   divider, nothing while a move, resize or divider drag is in progress;
//! - it never reaches a client through another client's grab: while the pointer or the keyboard
//!   is held by a surface that is not the leased window's client, it is dropped (a key waits a
//!   short bounded time for the keyboard to come back, `agent_wait`);
//! - it never lands on the shell's Overlay layer (a sheet that is up, or still closing after the
//!   person's answer to it): a pointer event there waits the same bounded time for the sheet to
//!   go, and is dropped if it stays;
//! - only keys, pointer motion, buttons and scrolling: no touch, gestures or switches.
//!
//! A press that goes on is noted as the lease's (`seat.agent_down`, [`note`]); one held back
//! later (the reach table refused it, or it only activated the window) is taken off again
//! ([`held_back`]). The release of a key or button goes on exactly when its press is noted, and
//! past the reach table (`deliver`): a client that was given a press gets its release, even after
//! the lease ended or the window lost focus, so nothing is left down; a release of anything else
//! reaches no one, and the person's own presses are never touched by a lease's.

use super::scene::{Hit, hit_test, stack};
use super::{divider, frames, overview};
use crate::state::State;
use casement_core::geometry::LogicalPoint;
use casement_core::input::{Edge, InputEvent, InputKind, Origin};
use casement_core::layer::{LayerLevel, LayerOwner};
use casement_core::{LeaseId, WindowId};
use casement_wayland::compositor::root_of;
use casement_wayland::seat::Pressed;
use smithay::desktop::{PopupKind, find_popup_root_surface};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

/// The rule that shut an agent event, named so it can be logged and told to the agent without
/// saying what the event was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Rule {
    /// A release of a press the lease never had delivered.
    NoPress,
    /// The lease ended, or its window is gone.
    NoLease,
    /// Not a key, motion, button or scroll.
    Kind,
    /// Mission Control has the input.
    Overview,
    /// A move or resize is in progress.
    Grab,
    /// A Split View divider is being dragged.
    Divider,
    /// The keyboard is on a surface outside the lease. A key waits for it ([`Gate::Wait`]).
    KeyFocusElsewhere,
    /// The pointer is grabbed by a surface outside the lease.
    PointerGrabElsewhere,
    /// The pointer event lands on the shell's Overlay layer. It waits for the layer to go
    /// ([`Gate::Wait`]).
    ShellOverlay,
    /// A press on the window's frame or the divider.
    Chrome,
    /// An event waited the bound for its rule to lift and it did not.
    HoldExpired,
}

/// Whether an agent event goes on to delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Gate {
    /// It goes on (the reach table still has its say).
    Open,
    /// It is dropped, by this rule.
    Shut(Rule),
    /// It is held for a short time: this rule may lift (a key while the keyboard is on a surface
    /// outside the lease, or a pointer event on the shell's sheet: the person's answer to the
    /// sheet closes it and gives the window back).
    Wait(Rule),
}

/// The gate for `event` of `lease`, with the pointer at `at`.
pub(super) fn gate(
    state: &State,
    lease: LeaseId,
    event: &InputEvent,
    at: Option<LogicalPoint>,
) -> Gate {
    if let Some((what, Edge::Release)) = pressed_of(&event.kind) {
        return match state.wl.seat.agent_down.holds(lease, what) {
            true => Gate::Open,
            false => Gate::Shut(Rule::NoPress),
        };
    }
    let Some(window) = state.core.model.leases().window_of(lease) else {
        return Gate::Shut(Rule::NoLease);
    };
    let rule = (!allowed_kind(&event.kind))
        .then_some(Rule::Kind)
        .or_else(|| overview::takes_input(state).then_some(Rule::Overview))
        .or_else(|| state.wl.seat.grab.is_some().then_some(Rule::Grab))
        .or_else(|| divider::is_dragging(state).then_some(Rule::Divider))
        .or_else(|| held_by_another(state, window, &event.kind))
        .or_else(|| on_shell_overlay(state, &event.kind, at).then_some(Rule::ShellOverlay))
        .or_else(|| on_compositor_chrome(state, &event.kind, at).then_some(Rule::Chrome));
    match rule {
        None => Gate::Open,
        Some(rule @ (Rule::KeyFocusElsewhere | Rule::ShellOverlay)) => Gate::Wait(rule),
        Some(rule) => Gate::Shut(rule),
    }
}

/// Notes a press of `lease` that went on through the gate; a release takes its press off.
pub(super) fn note(state: &mut State, lease: LeaseId, kind: &InputKind) {
    let down = std::mem::take(&mut state.wl.seat.agent_down);
    state.wl.seat.agent_down = match pressed_of(kind) {
        Some((what, Edge::Press)) => down.pressed(lease, what),
        Some((what, Edge::Release)) => down.held_back(lease, what),
        None => down,
    };
}

/// Takes an agent press that was not delivered after all off its lease, so its release reaches
/// no one. Whether `origin` was a lease's (the person's own presses are `seat.taken`'s).
pub(super) fn held_back(state: &mut State, origin: Origin, what: Pressed) -> bool {
    let Origin::Agent(lease) = origin else {
        return false;
    };
    let down = std::mem::take(&mut state.wl.seat.agent_down);
    state.wl.seat.agent_down = down.held_back(lease, what);
    true
}

fn pressed_of(kind: &InputKind) -> Option<(Pressed, Edge)> {
    match kind {
        InputKind::Key { code, edge } => Some((Pressed::Key(*code), *edge)),
        InputKind::Button { button, edge } => Some((Pressed::Button(*button), *edge)),
        _ => None,
    }
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

/// The window `surface` belongs to: the window of its subsurface tree's root, or of the popup
/// chain that root opens from. None for a layer, a lock surface or a surface with no role.
fn window_holding(state: &State, surface: &WlSurface) -> Option<WindowId> {
    let shell = &state.wl.xdg_shell;
    let root = root_of(surface);
    shell.window_of_surface(&root).or_else(|| {
        let popup = shell
            .popup_surfaces
            .values()
            .find(|popup| *popup.wl_surface() == root)?;
        let opener = find_popup_root_surface(&PopupKind::Xdg(popup.clone())).ok()?;
        shell.window_of_surface(&root_of(&opener))
    })
}

/// The rule that applies when the pointer (for pointer events) or the keyboard (for keys) is
/// grabbed by, or focused on, a surface that is not `leased` or one of its popups: a popup grab, a drag, an
/// input method's, another window's or another app's implicit grab would carry the event there.
fn held_by_another(state: &State, leased: WindowId, kind: &InputKind) -> Option<Rule> {
    let seat = &state.wl.seat;
    let elsewhere = |surface: Option<WlSurface>| {
        surface.is_some_and(|surface| window_holding(state, &surface) != Some(leased))
    };
    match kind {
        InputKind::Key { .. } => {
            elsewhere(seat.keyboard.current_focus()).then_some(Rule::KeyFocusElsewhere)
        }
        _ => (seat.pointer.is_grabbed()
            && elsewhere(seat.pointer.current_focus().and_then(Into::into)))
        .then_some(Rule::PointerGrabElsewhere),
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

/// Whether a pointer event lands on a layer of the shell's at the Overlay level: where it moves
/// to (`MotionTo`), or the pointer `at` (a relative move starts there, a button or a scroll
/// is there).
fn on_shell_overlay(state: &State, kind: &InputKind, at: Option<LogicalPoint>) -> bool {
    let point = match kind {
        InputKind::MotionTo { at } => Some(*at),
        InputKind::Motion { .. } | InputKind::Button { .. } | InputKind::Axis(_) => at,
        _ => None,
    };
    let Some(point) = point else {
        return false;
    };
    let Some((Hit::Layer(layer), _)) = hit_test(&stack(state), point) else {
        return false;
    };
    state
        .core
        .model
        .layers()
        .get(&layer)
        .is_some_and(|spec| spec.level == LayerLevel::Overlay && spec.owner == LayerOwner::Shell)
}
