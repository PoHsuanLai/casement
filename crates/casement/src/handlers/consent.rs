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
use casement_wayland::consent::{ConsentHandler, Ledger};
use casement_wayland::quire::shell_permission;
use casement_wayland::{gamma, output_management, output_power, virtual_keyboard};
use casement_wayland::{wlr_toplevel, workspace};

impl ConsentHandler for State {
    fn consent_ledger(&self) -> &Ledger {
        self.wl.clients.ledger()
    }

    fn consent_events(&mut self, events: Vec<Event>) {
        core_loop::feed(self, events);
    }
}

/// How a use of `power` by `client` came out: the ledger first, so a request made while the
/// protocols answer sees the new word, then each protocol of the power.
///
/// The clipboard managers, the input method and the window list of `ext_foreign_toplevel_list`
/// are smithay's own dispatch and have no place to hold a request (FINDINGS): the ledger knows
/// their answers, nothing waits on them yet.
pub(crate) fn resolved(state: &mut State, client: ClientKey, power: Power, outcome: PowerOutcome) {
    state.wl.clients.ledger().resolve(client, power, outcome);
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

/// `Effect::AskPermission`: the question goes to the shell.
pub(crate) fn ask(
    state: &mut State,
    ask: AskId,
    power: Power,
    subject: &AskSubject,
    memory: Memory,
) {
    shell_permission::apply_ask(&mut state.wl.quire_shell, ask, power, subject, memory);
}

/// `Effect::WithdrawAsk`: the question is taken back from the shell.
pub(crate) fn withdraw(state: &mut State, ask: AskId) {
    shell_permission::apply_withdraw(&mut state.wl.quire_shell, ask);
}
