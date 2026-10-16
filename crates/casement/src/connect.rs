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
use crate::core_loop::{feed, tty::desktop_extent};
use casement_agent::{
    CallerUnit, HandOut, RefuseWhy, RemoteEvent, RemoteKinds, hand_out, open_remote_eis,
    portal_may_connect,
};
use casement_core::Event;
use casement_core::input::{InputEvent, PortalSessionId};
use casement_ipc::dbus::{
    ConnectAsk, ConnectCall, ConnectRefusal, ConnectReply, EndAsk, RemoteAsk,
};
use casement_wayland::ClientState;
use porter_core::PeerIdentity;
use smithay::reexports::calloop::channel::{Channel, Event as ChannelEvent};
use smithay::reexports::calloop::{Error as LoopError, LoopHandle};

/// Starts answering `calls` on `handle`'s loop.
pub fn attach(
    handle: &LoopHandle<'static, State>,
    calls: Channel<ConnectCall>,
) -> Result<(), LoopError> {
    handle
        .insert_source(calls, |event, (), state: &mut State| {
            if let ChannelEvent::Msg(call) = event {
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
    if let Some(remote) = ask.remote {
        return answer_remote(state, remote, &ask.caller_cgroup);
    }
    if let Some(end) = ask.end {
        return answer_end(state, end, &ask.caller_cgroup);
    }
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

/// Whether the caller's control group is the portal's unit: the one caller that may open or end
/// a remote EIS socket.
fn portal_only(caller_cgroup: &str) -> Result<(), ConnectRefusal> {
    match CallerUnit::of_cgroup(caller_cgroup) {
        None => Err(ConnectRefusal::UnknownUnit),
        Some(unit) if !portal_may_connect(&unit) => Err(ConnectRefusal::RoleNotAllowed),
        Some(_) => Ok(()),
    }
}

/// Decides one `ConnectEis`: only the portal's unit may have a remote EIS socket. On a grant
/// the server end becomes an event-loop source whose input is fed to the model tagged as the
/// remote origin of portal session `remote.session`, with a device slot of its own (minted
/// here, one per connection: CASW-19); the client end is the reply.
pub fn answer_remote(state: &mut State, remote: RemoteAsk, caller_cgroup: &str) -> ConnectReply {
    if let Err(why) = portal_only(caller_cgroup) {
        return ConnectReply::Refused(why);
    }
    let Some(slot) = state.agent.remote.mint() else {
        tracing::warn!("every remote input slot has been used");
        return ConnectReply::Refused(ConnectRefusal::Unavailable);
    };
    let extent = desktop_extent(state);
    let grant = match open_remote_eis(
        PortalSessionId(remote.session),
        slot,
        RemoteKinds::from_wire(remote.devices),
        extent,
    ) {
        Ok(grant) => grant,
        Err(error) => {
            tracing::warn!(%error, "a remote input socket could not be made");
            return ConnectReply::Refused(ConnectRefusal::Unavailable);
        }
    };
    let inserted = state.event_loop.insert_source(
        grant.source,
        move |event, (), state: &mut State| match event {
            RemoteEvent::Input(input) => {
                let at = state.clock.now();
                deliver_remote(state, input.at(at));
            }
            RemoteEvent::Ended => state.agent.remote.closed(slot),
            _ => {}
        },
    );
    match inserted {
        Ok(token) => {
            state.agent.remote.opened(grant.handle, token);
            ConnectReply::Granted(grant.client)
        }
        Err(error) => {
            tracing::warn!(%error, "a remote input source could not be inserted");
            ConnectReply::Refused(ConnectRefusal::Unavailable)
        }
    }
}

/// Decides one `EndEis`: only the portal's unit may end a session. Every socket handed out for
/// `end.session` is taken out of the loop (its server end closes, so the app is disconnected and
/// sends nothing more), and the keys and buttons it still held are released. A session with no
/// open socket is not an error: the portal may end one that already hung up.
pub fn answer_end(state: &mut State, end: EndAsk, caller_cgroup: &str) -> ConnectReply {
    if let Err(why) = portal_only(caller_cgroup) {
        return ConnectReply::Refused(why);
    }
    for (handle, token) in state.agent.remote.end(PortalSessionId(end.session)) {
        state.event_loop.remove(token);
        for release in handle.release() {
            let at = state.clock.now();
            deliver_remote(state, release.at(at));
        }
    }
    ConnectReply::Ended
}

/// One input event from a remote session into the model, by the path every input takes. Its
/// origin was set where it entered (`casement-agent`'s EIS) and is not changed here.
pub fn deliver_remote(state: &mut State, event: InputEvent) {
    feed(state, vec![Event::Input(event)]);
}
