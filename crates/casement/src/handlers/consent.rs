//! Per-app consent for the privileged globals, the state's side (`casement_wayland::consent`):
//! the ledger the protocol modules read, the events a first use sends, and what the model's
//! three consent effects do on the wire.
//!
//! `Effect::Resolve` is the answer to a use: it updates the ledger, then each protocol of the
//! power replays what it held, gives its own denial or ends the use. `Effect::AskPermission`
//! and `Effect::WithdrawAsk` are the questions to the person, carried by `quire_shell_v1`
//! (`permission_ask`, `permission_withdraw`); the answer comes back as
//! `Event::PermissionAnswered`.

use crate::core_loop;
use crate::state::State;
use casement_core::privilege::{AskSubject, PowerOutcome};
use casement_core::{ClientKey, Event};
use casement_protocols::vocab::{AskId, Memory, Power};
use casement_wayland::consent::{ConsentHandler, Ledger, release_gate};
use casement_wayland::handler::EventSink;
use casement_wayland::quire::shell_permission;
use casement_wayland::{gamma, output_management, output_power, virtual_keyboard};
use casement_wayland::{wlr_toplevel, workspace};
use smithay::reexports::wayland_server::backend::ClientId;
use smithay::wayland::input_method::InputMethodSeat;

impl EventSink for State {
    fn feed(&mut self, events: Vec<Event>) {
        core_loop::feed(self, events);
    }
}

impl ConsentHandler for State {
    fn consent_ledger(&self) -> &Ledger {
        self.wl.clients.ledger()
    }
}

/// How a use of `power` by `client` came out: the ledger first, so a request made while the
/// protocols answer sees the new word, then each protocol of the power.
///
/// The clipboard managers, the input method and the window list of `ext_foreign_toplevel_list`
/// are smithay's own dispatch: it holds what a waiting client created and `settle_served` tells
/// it the answer.
pub(crate) fn resolved(state: &mut State, client: ClientKey, power: Power, outcome: PowerOutcome) {
    state.wl.clients.ledger().resolve(client, power, outcome);
    settle_served(state, client, power, outcome);
    match power {
        Power::Screen => {
            super::screencopy::resolved(state, client, outcome);
            super::image_capture::resolved(state, client, outcome);
        }
        Power::Keys => virtual_keyboard::on_resolved(state, client, outcome),
        Power::Lock => super::session_lock::resolved(state, client, outcome),
        Power::Displays => {
            output_management::on_resolved(state, client, outcome);
            output_power::on_resolved(state, client, outcome);
            gamma::on_resolved(state, client, outcome);
        }
        Power::Windows => {
            wlr_toplevel::on_resolved(state, client, outcome);
            workspace::on_resolved(&mut state.wl.workspace, client, outcome);
        }
        Power::TypingHelp | Power::Clipboard => {}
    }
}

/// Settles what smithay holds for `client` in the protocols it serves whole: held objects are
/// opened on an allow and refused on a refusal. A power taken back also ends what smithay
/// already registered for the client ([`end_served`]): the clipboard devices, the input method
/// and the window lists are finished or made unavailable, and the objects made after it are
/// refused until the next use asks again. Nothing happens for a client that never had anything
/// on hold, unless its power was taken back (it may hold live objects without ever having waited).
fn settle_served(state: &mut State, client: ClientKey, power: Power, outcome: PowerOutcome) {
    let ledger = state.wl.clients.ledger();
    let known = match outcome {
        PowerOutcome::Revoked => ledger.connection(client),
        PowerOutcome::Allowed | PowerOutcome::Denied => ledger.parked(client),
    };
    let Some(id) = known else {
        return;
    };
    let gate = release_gate(outcome);
    let display = state.display.clone();
    match power {
        Power::Clipboard => {
            state.wl.ext_data_control.state.release(&display, &id, gate);
            state.wl.wlr_data_control.state.release(&display, &id, gate);
        }
        Power::TypingHelp => state.wl.input_method.state.release(&display, &id, gate),
        Power::Windows => state
            .wl
            .foreign_toplevel_list
            .state
            .release(&display, &id, gate),
        Power::Screen | Power::Keys | Power::Lock | Power::Displays => {}
    }
    match outcome {
        PowerOutcome::Revoked => end_served(state, power, &id),
        PowerOutcome::Allowed | PowerOutcome::Denied => {}
    }
}

/// Ends the objects smithay registered for the connection `id` in the protocols of `power`
/// that it serves whole: the data devices of both clipboard managers are finished, the input
/// method is told `unavailable` and leaves the seat, the window lists are finished. A
/// selection the client already set stays until another replaces it.
fn end_served(state: &mut State, power: Power, id: &ClientId) {
    match power {
        Power::Clipboard => {
            state.wl.ext_data_control.state.revoke::<State>(id);
            state.wl.wlr_data_control.state.revoke::<State>(id);
        }
        Power::TypingHelp => {
            let method = state.wl.seat.seat.input_method().clone();
            method.revoke::<State>(state, id);
        }
        Power::Windows => state.wl.foreign_toplevel_list.state.revoke(id),
        Power::Screen | Power::Keys | Power::Lock | Power::Displays => {}
    }
}

/// `Effect::AskPermission`: the question goes to the shell.
pub(crate) fn ask(
    state: &mut State,
    ask: AskId,
    power: Power,
    subject: &AskSubject,
    memory: Memory,
) {
    let now = state.clock.now();
    shell_permission::apply_ask(&mut state.wl.quire_shell, ask, power, subject, memory, now);
}

/// `Effect::WithdrawAsk`: the question is taken back from the shell.
pub(crate) fn withdraw(state: &mut State, ask: AskId) {
    shell_permission::apply_withdraw(&mut state.wl.quire_shell, ask);
}
