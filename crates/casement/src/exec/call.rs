//! A binding's D-Bus call (A4, P1-Z; filled by P1-I1): `Effect::CallBinding` goes to the
//! injected `ShellCalls` seam. This file names the call and never zbus; failures that come
//! back later are logged here.

use crate::state::State;
use casement_core::input::DbusCall;
use casement_ipc::shell_calls::ShellCalls;

/// Sends `call` through `state.seams.shell_calls` within the frame the binding fired in. A
/// failure is logged and degrades; it never stops the loop.
pub(super) fn call_binding(state: &mut State, call: DbusCall) {
    if let Err(error) = state.seams.shell_calls.call(&call) {
        tracing::warn!(%error, method = %call.method, "a binding's call was not sent");
    }
    for failed in state.seams.shell_calls.failures() {
        tracing::warn!(
            error = %failed.why,
            method = %failed.call.method,
            "a binding's call failed"
        );
    }
}
