//! `org.quire.Compositor1.Connect` in the event loop (P2-Z): the D-Bus object proves each caller
//! on the bus thread and forwards the call down a calloop channel, and [`answer`] decides it with
//! the `State` in hand: the role the caller proved against the role it asked for
//! (`casement_agent::hand_out`), and the server end of the new socket inserted into the display
//! as a client of that role.
//!
//! `casement-agent` makes the socket pair and the decision; only this crate can build a
//! client's data (its key, its gone queue), so the insertion is here. Frozen at P2-Z; filled
//! by P2-Q2.

use crate::State;
use crate::clients::{Standing, insert_client};
use crate::core_loop::{feed, tty::desktop_extent};
use casement_agent::{
    HandOut, RefuseWhy, RemoteEvent, RemoteHandle, RemoteKinds, RoleName, hand_out,
    open_remote_eis, portal_may_connect,
};
use casement_core::Event;
use casement_core::input::{InputEvent, PortalSessionId};
use casement_ipc::dbus::{
    ConnectAsk, ConnectCall, ConnectRefusal, ConnectReply, EndAsk, GoneAsk, RemoteAsk,
};
use casement_wayland::ClientState;
use porter_core::PeerIdentity;
use smithay::reexports::calloop::channel::{Channel, Event as ChannelEvent};
use smithay::reexports::calloop::{Error as LoopError, LoopHandle, RegistrationToken};

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

/// Decides one `Connect`: the role `ask.caller` proved against `ask.role`; on a grant the
/// server end of a new socket pair is inserted as a client with that role (the model hears
/// `ClientConnected`), and the client end is the reply.
pub fn answer(state: &mut State, ask: ConnectAsk) -> ConnectReply {
    if let Some(gone) = ask.gone {
        return answer_gone(state, &gone);
    }
    if let Some(remote) = ask.remote {
        return answer_remote(state, remote, ask.owner, &ask.caller.role);
    }
    if let Some(end) = ask.end {
        return answer_end(state, end, &ask.caller.role);
    }
    let handed = match hand_out(&ask.role, &ask.caller.role) {
        HandOut::Granted(handed) => handed,
        HandOut::Refused(why) => return ConnectReply::Refused(refusal_of(why)),
    };
    // A handed-out connection is the shell's or the agent tier's, trusted by how it was
    // obtained; its peer stays unproven (the app it belongs to is `ask.caller.app`, kept below).
    let client = match insert_client(
        state,
        handed.server,
        handed.role,
        PeerIdentity::Unproven,
        Standing::Unjudged,
    ) {
        Ok(client) => client,
        Err(error) => {
            tracing::warn!(%error, "a handed-out connection could not be inserted");
            return ConnectReply::Refused(ConnectRefusal::Unavailable);
        }
    };
    if let Some(data) = ClientState::of(&client) {
        state.agent.handed.insert(data.key, ask.caller.app.clone());
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

/// Whether the caller holds the portal's role: the one caller that may open or end a remote EIS
/// socket (the bus thread proved the role; this is the second line).
fn portal_only(caller: &RoleName) -> Result<(), ConnectRefusal> {
    match portal_may_connect(caller) {
        true => Ok(()),
        false => Err(ConnectRefusal::RoleNotAllowed),
    }
}

/// Decides one `ConnectEis`: only the portal may have a remote EIS socket. On a grant
/// the server end becomes an event-loop source whose input is fed to the model tagged as the
/// remote origin of portal session `remote.session`, with a device slot of its own (minted
/// here, one per connection: CASW-19); the client end is the reply.
pub fn answer_remote(
    state: &mut State,
    remote: RemoteAsk,
    owner: Option<String>,
    caller: &RoleName,
) -> ConnectReply {
    if let Err(why) = portal_only(caller) {
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
            state.agent.remote.opened(grant.handle, token, owner);
            ConnectReply::Granted(grant.client)
        }
        Err(error) => {
            tracing::warn!(%error, "a remote input source could not be inserted");
            ConnectReply::Refused(ConnectRefusal::Unavailable)
        }
    }
}

/// Decides one `EndEis`: only the portal may end a session. Every socket handed out for
/// `end.session` is taken out of the loop (its server end closes, so the app is disconnected and
/// sends nothing more), and the keys and buttons it still held are released. A session with no
/// open socket is not an error: the portal may end one that already hung up.
pub fn answer_end(state: &mut State, end: EndAsk, caller: &RoleName) -> ConnectReply {
    if let Err(why) = portal_only(caller) {
        return ConnectReply::Refused(why);
    }
    let ended = state.agent.remote.end(PortalSessionId(end.session));
    close(state, ended);
    ConnectReply::Ended
}

/// The portal's bus connection `gone.owner` left the bus (a restart, a crash): every remote
/// socket it asked for is closed and the keys and buttons held are let go, so an app does not
/// keep injecting input into a session nobody is accounting for any more (NEW-PI-8).
pub fn answer_gone(state: &mut State, gone: &GoneAsk) -> ConnectReply {
    let ended = state.agent.remote.end_owner(&gone.owner);
    close(state, ended);
    ConnectReply::Ended
}

/// Takes `ended` out of the loop and lets go of what each held.
fn close(state: &mut State, ended: Vec<(RemoteHandle, RegistrationToken)>) {
    for (handle, token) in ended {
        state.event_loop.remove(token);
        for release in handle.release() {
            let at = state.clock.now();
            deliver_remote(state, release.at(at));
        }
    }
}

/// One input event from a remote session into the model, by the path every input takes. Its
/// origin was set where it entered (`casement-agent`'s EIS) and is not changed here.
pub fn deliver_remote(state: &mut State, event: InputEvent) {
    feed(state, vec![Event::Input(event)]);
}
