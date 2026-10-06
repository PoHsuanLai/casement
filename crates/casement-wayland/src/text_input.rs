//! `zwp_text_input_manager_v3`: text fields. smithay ties text-input focus to keyboard focus;
//! the first key after `enable` must reach the input method, not the document (SP1 A5).

use crate::seat::KeyDelivery;
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::Event;
use casement_core::time::Mono;
use smithay::input::{Seat, SeatHandler};
use smithay::reexports::wayland_protocols::wp::text_input::zv3::server::{
    zwp_text_input_manager_v3::ZwpTextInputManagerV3, zwp_text_input_v3::ZwpTextInputV3,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch, Resource};
use smithay::wayland::GlobalData;
use smithay::wayland::input_method::InputMethodSeat;
use smithay::wayland::text_input::{TextInputManagerState, TextInputSeat, TextInputUserData};

mod gate;

pub use gate::{FieldFacts, GrabFacts, HOLD_LIMIT, ImeFacts, KeyGate, MethodFacts};

/// `zwp_text_input_manager_v3`.
#[derive(Debug)]
pub struct TextInputPart {
    /// smithay's state.
    pub state: TextInputManagerState,
    /// Keys waiting for the input method to take the keyboard (the activation gate).
    pub gate: KeyGate,
}

impl TextInputPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> TextInputPart
    where
        D: GlobalDispatch<ZwpTextInputManagerV3, GlobalData>
            + Dispatch<ZwpTextInputManagerV3, GlobalData>
            + Dispatch<ZwpTextInputV3, TextInputUserData>
            + 'static,
    {
        let _ = role_filter;
        TextInputPart {
            state: TextInputManagerState::new::<D>(display),
            gate: KeyGate::new(),
        }
    }
}

/// A text field enabled text input. smithay takes the field's requests itself and relays them
/// to the input method, and the model has no text-field state, so this is no event; it opens
/// a new activation for the gate (the wait for the input method's grab starts afresh).
pub fn on_enable(part: &mut TextInputPart, surface: &WlSurface) -> Vec<Event> {
    let _ = surface;
    part.gate.restart();
    Vec::new()
}

/// A text field committed its state (surrounding text, content type, cursor rectangle).
/// smithay relays it to the input method; the model has no text-field state, so no events.
pub fn on_commit_state(part: &mut TextInputPart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    Vec::new()
}

/// What the seat's input method and text field say right now (the gate's facts).
pub fn facts_of<D: SeatHandler + 'static>(seat: &Seat<D>) -> ImeFacts {
    let method = seat.input_method();
    let mut field = FieldFacts::Inactive;
    seat.text_input()
        .with_active_text_input(|_, _| field = FieldFacts::Active);
    ImeFacts {
        method: if method.has_instance() {
            MethodFacts::Bound
        } else {
            MethodFacts::Absent
        },
        field,
        grab: if method.keyboard_grabbed() {
            GrabFacts::Grabbed
        } else {
            GrabFacts::Free
        },
    }
}

/// `key` arrived: the keys to deliver now, in order (the key itself, unless an input method
/// is about to take the keyboard; then it waits and comes out of [`settle_keys`]). Deliver
/// them through smithay's keyboard `input`, which honours the input method's grab.
pub fn admit_key<D: SeatHandler + 'static>(
    part: &mut TextInputPart,
    seat: &Seat<D>,
    key: KeyDelivery,
    now: Mono,
) -> Vec<KeyDelivery> {
    part.gate.admit(facts_of(seat), key, now)
}

/// A dispatch round ended or the hold timer fired: the waiting keys that may go now.
pub fn settle_keys<D: SeatHandler + 'static>(
    part: &mut TextInputPart,
    seat: &Seat<D>,
    now: Mono,
) -> Vec<KeyDelivery> {
    part.gate.settle(facts_of(seat), now)
}

/// Moves text-input focus with keyboard focus. smithay does this itself when keyboard focus
/// moves to a surface; this reconciles the two when they differ (focus moved without it, or
/// the surface is gone): `leave` for the old focus, then the new focus is set *before* `enter`
/// is sent, so the field's immediate `enable` is not discarded as coming from an unfocused
/// client (the SP1 A5 failure).
pub fn apply_text_focus<D: SeatHandler + 'static>(
    seat: &Seat<D>,
    focus: Option<&WlSurface>,
) -> Result<(), WaylandFault> {
    if focus.is_some_and(|surface| !surface.is_alive()) {
        return Err(WaylandFault::SurfaceGone);
    }
    let text = seat.text_input();
    if text.focus().as_ref() == focus {
        return Ok(());
    }
    // `enter` and `leave` follow focus with or without an input method (the fork's rule: an IME
    // usually starts after the apps).
    if text.focus().is_some() {
        text.leave();
    }
    text.set_focus(focus.cloned());
    if focus.is_some() {
        text.enter();
    }
    Ok(())
}

/// Sends a preedit to the focused, enabled field and closes the batch with `done`.
pub fn apply_preedit<D: SeatHandler + 'static>(
    seat: &Seat<D>,
    text: &str,
    cursor: (i32, i32),
) -> Result<(), WaylandFault> {
    with_field(seat, |field| {
        field.preedit_string(Some(text.to_owned()), cursor.0, cursor.1);
    })
}

/// Commits text to the focused, enabled field and closes the batch with `done`.
pub fn apply_commit_string<D: SeatHandler + 'static>(
    seat: &Seat<D>,
    text: &str,
) -> Result<(), WaylandFault> {
    with_field(seat, |field| field.commit_string(Some(text.to_owned())))
}

/// Runs `send` on the active text input of the focused client, then `done` with its serial;
/// no such field is a vanished surface.
fn with_field<D: SeatHandler + 'static>(
    seat: &Seat<D>,
    send: impl Fn(&ZwpTextInputV3),
) -> Result<(), WaylandFault> {
    let handle = seat.text_input();
    let mut reached = Err(WaylandFault::SurfaceGone);
    handle.with_active_text_input(|field, _| {
        send(field);
        reached = Ok(());
    });
    handle.done(false);
    reached
}
