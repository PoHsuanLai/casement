//! `org.quire.Compositor1.Connect` in the event loop (P2-Z): the D-Bus object forwards every
//! call down a calloop channel and [`answer`] decides it with the `State` in hand: the caller's
//! unit against the role it asked for (`casement_agent::hand_out`), and the server end of the
//! new socket inserted into the display as a client of that role.
//!
//! `casement-agent` makes the socket pair and the decision; only this crate can build a
//! client's data (its key, its gone queue), so the insertion is here. Frozen at P2-Z; filled
//! by P2-Q2.

use crate::State;
use crate::clients::insert_client;
use casement_agent::{CallerUnit, HandOut, RefuseWhy, hand_out};
use casement_ipc::dbus::{ConnectAsk, ConnectCall, ConnectRefusal, ConnectReply};
use casement_wayland::ClientState;
use porter_core::PeerIdentity;
use smithay::reexports::calloop::channel::{Channel, Event};
use smithay::reexports::calloop::{Error as LoopError, LoopHandle};

/// Starts answering `calls` on `handle`'s loop.
pub fn attach(
    handle: &LoopHandle<'static, State>,
    calls: Channel<ConnectCall>,
) -> Result<(), LoopError> {
    handle
        .insert_source(calls, |event, (), state: &mut State| {
            if let Event::Msg(call) = event {
                let reply = answer(state, call.ask);
                // A caller that gave up has dropped the receiver; nothing to tell it.
                let _ = call.reply.send(reply);
            }
        })
        .map_err(LoopError::from)?;
    Ok(())
}

/// Decides one `Connect`: the unit in `ask.caller_cgroup` against `ask.role`; on a grant the
/// server end of a new socket pair is inserted as a client with that role (the model hears
/// `ClientConnected`), and the client end is the reply.
pub fn answer(state: &mut State, ask: ConnectAsk) -> ConnectReply {
    let Some(unit) = CallerUnit::of_cgroup(&ask.caller_cgroup) else {
        return ConnectReply::Refused(ConnectRefusal::UnknownUnit);
    };
    let handed = match hand_out(&ask.role, &unit) {
        HandOut::Granted(handed) => handed,
        HandOut::Refused(why) => return ConnectReply::Refused(refusal_of(why)),
    };
    // The peer stays unproven until porter can prove a service (accountd's unit check).
    let client = match insert_client(state, handed.server, handed.role, PeerIdentity::Unproven) {
        Ok(client) => client,
        Err(error) => {
            tracing::warn!(%error, "a handed-out connection could not be inserted");
            return ConnectReply::Refused(ConnectRefusal::Unavailable);
        }
    };
    if let Some(data) = ClientState::of(&client) {
        state.agent.handed.insert(data.key, unit);
    }
    ConnectReply::Granted(handed.client)
}

/// What the caller is told for a hand-out `hand_out` refused. A socket pair that could not be
/// made is logged here (`casement-agent` has no logger).
fn refusal_of(why: RefuseWhy) -> ConnectRefusal {
    match why {
        RefuseWhy::UnknownUnit => ConnectRefusal::UnknownUnit,
        RefuseWhy::RoleNotAllowed => ConnectRefusal::RoleNotAllowed,
        _ => {
            tracing::warn!("a socket pair for a handed-out connection could not be made");
            ConnectRefusal::Unavailable
        }
    }
}
