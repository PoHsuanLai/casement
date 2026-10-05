//! The text-input gate's clock: keys held for the input method's grab are let go when a
//! dispatch round ends (the grab may have arrived) and when the hold timer fires (a method that
//! never grabs must not stall typing).

use super::deliver::{arm_ime_hold, deliver_keys};
use crate::state::State;
use casement_core::Event;
use casement_core::time::TimerKey;
use casement_wayland::text_input;

/// The hold timer fired in `events`: the keys it released go.
pub(super) fn hear(state: &mut State, events: &[Event]) {
    if events
        .iter()
        .any(|event| matches!(event, Event::TimerFired(TimerKey::ImeHold)))
    {
        settle(state);
    }
}

/// The waiting keys that may go now are delivered, in order; the timer is armed again if some
/// still wait.
pub(super) fn settle(state: &mut State) {
    let seat = state.wl.seat.seat.clone();
    let now = state.clock.now();
    let ready = text_input::settle_keys(&mut state.wl.text_input, &seat, now);
    deliver_keys(state, &seat, ready);
    arm_ime_hold(state);
}
