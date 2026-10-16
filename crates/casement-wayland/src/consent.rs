//! Per-app consent at the wire: how a protocol module holds, replays or refuses a request that
//! needs one of the seven powers (`casement_core::privilege`).
//!
//! ```text
//! request ──► admit(client, power) ──► Go ────► carry it out
//!                                  ├─► Refuse ► the protocol's own denial
//!                                  └─► Hold ──► kept in the module's Held, until
//!                                               Effect::Resolve:  Allowed ► replay
//!                                                                 Denied  ► the denial
//!                                                                 Revoked ► end the use
//! ```
//!
//! [`admit`] looks the client up in the [`Ledger`] (the latest word per client and power). A
//! client with no word, or a refusal, is asked of the model: the first use sends
//! `Event::PrivilegeUse` (after `Event::ClientStanding` when the program behind the connection
//! changed its image since accept), and the model answers at once or after the person has
//! answered. The modules keep their own held requests, at most
//! [`HELD_REQUESTS`](casement_core::privilege::HELD_REQUESTS) per client ([`Held`]); the
//! composition root's `Effect::Resolve` arm updates the ledger and calls each module's replay.

mod farewell;
mod held;
mod ledger;
mod watch;

pub use farewell::{Farewell, later as finish_later, send_queued as send_farewells};
pub use held::{Held, Mortal, Pushed};
pub use ledger::{Admission, Ledger, Word};
pub use watch::{ImageWatch, Unwatched};

use crate::client::ClientState;
use casement_core::ClientKey;
use casement_core::Event;
use casement_core::output::Requester;
use casement_core::privilege::PowerOutcome;
use casement_protocols::vocab::Power;
use ledger::Begin;
use smithay::reexports::wayland_server::Client;
use smithay::wayland::Gate;

/// What the composition root provides to every module that gates a power.
pub trait ConsentHandler: 'static {
    /// The latest word per client and power.
    fn consent_ledger(&self) -> &Ledger;
    /// Feeds the events a use produced into the model (`core_loop::feed`). The model may
    /// resolve the use before this returns.
    fn consent_events(&mut self, events: Vec<Event>);
}

/// What `client`'s request for `power` does now. A client casement did not insert is refused
/// (it has no standing).
pub fn admit<D: ConsentHandler>(state: &mut D, client: &Client, power: Power) -> Admission {
    match ClientState::of(client) {
        Some(data) => admit_key(state, data, power),
        None => Admission::Refuse,
    }
}

/// [`admit`] for a client whose data the caller already has.
fn admit_key<D: ConsentHandler>(state: &mut D, data: &ClientState, power: Power) -> Admission {
    let key = data.key;
    match state.consent_ledger().begin(key, power) {
        Begin::Settled(admission) => admission,
        Begin::Fresh => {
            state.consent_events(first_use(data, power));
            state.consent_ledger().admission(key, power)
        }
    }
}

/// The events of a client's first use of `power`: the program's new standing when it executed
/// another image since it connected (so what it is allowed is decided for the program it now
/// is), then the use itself.
fn first_use(data: &ClientState, power: Power) -> Vec<Event> {
    let moved = data.image.moved().map(|standing| Event::ClientStanding {
        client: data.key,
        standing,
    });
    moved
        .into_iter()
        .chain([Event::PrivilegeUse {
            client: data.key,
            power,
        }])
        .collect()
}

/// [`admit`] for the protocols smithay serves whole (the clipboard managers, the input method,
/// the window list of `ext_foreign_toplevel_list`): the answer in smithay's words. A client
/// that must wait is parked in the [`Ledger`] under its connection, so
/// `Effect::Resolve` can settle what smithay holds for it
/// ([`release_gate`]). smithay keeps at most
/// [`MAX_HELD_PER_CLIENT`](smithay::wayland::gate::MAX_HELD_PER_CLIENT) objects per client and
/// protocol and refuses the rest itself, so casement keeps no count of its own for them.
pub fn gate<D: ConsentHandler>(state: &mut D, client: &Client, power: Power) -> Gate {
    match admit(state, client, power) {
        Admission::Go => Gate::Open,
        Admission::Refuse => Gate::Refuse,
        Admission::Hold => {
            if let Some(key) = key_of(client) {
                state.consent_ledger().park(key, client.id());
            }
            Gate::Hold
        }
    }
}

/// What smithay does with the objects it held for a client once the model answered `outcome`:
/// an allowed use opens them; a refusal, or a power taken back, refuses them.
pub fn release_gate(outcome: PowerOutcome) -> Gate {
    match outcome {
        PowerOutcome::Allowed => Gate::Open,
        PowerOutcome::Denied | PowerOutcome::Revoked => Gate::Refuse,
    }
}

/// Whether `client` may use `power` now, without asking: the guard for requests on objects
/// that were made while the power was allowed (a handle, a control).
pub fn allowed<D: ConsentHandler>(state: &D, client: &Client, power: Power) -> Admission {
    match ClientState::of(client) {
        Some(data) => state.consent_ledger().admission(data.key, power),
        None => Admission::Refuse,
    }
}

/// The key of `client`, when casement inserted it.
pub fn key_of(client: &Client) -> Option<ClientKey> {
    ClientState::of(client).map(|data| data.key)
}

/// Who a request from `client` is from: the shell's own connection, or a program. A client
/// casement did not insert is a program.
pub fn requester_of(client: &Client) -> Requester {
    ClientState::of(client).map_or(Requester::Program, |data| Requester::of(&data.role))
}
