//! A binding's D-Bus call (A4, P1-Z; filled by P1-I1): `Effect::CallBinding` goes to the
//! injected `ShellCalls` seam. This file names the call and never zbus; failures that come
//! back later are logged here.

use crate::state::State;
use casement_core::input::DbusCall;

/// Sends `call` through `state.seams.shell_calls` within the frame the binding fired in. A
/// failure is logged and degrades; it never stops the loop.
pub(super) fn call_binding(state: &mut State, call: DbusCall) {
    let _ = (state, call);
    todo!("P1-I1: exec::call::call_binding")
}
