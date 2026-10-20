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
    HandOut, RefuseWhy, RemoteEvent, RemoteFor, RemoteHandle, RemoteKinds, RoleName,
    agent_may_lease, hand_out, open_remote_eis, portal_may_connect,
};
use casement_core::input::{InputEvent, PortalSessionId};
use casement_core::{Event, LeaseId};
use casement_ipc::dbus::{
    ConnectAsk, ConnectCall, ConnectRefusal, ConnectReply, EndAsk, EndLeaseAsk, GoneAsk, LeaseAsk,
    RemoteAsk,
};
use casement_wayland::ClientState;
use casement_wayland::foreign_toplevel_list::window_of_identifier;
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
    if let Some(lease) = ask.lease {
        return answer_lease(state, lease, ask.owner, &ask.caller.role);
    }
    if let Some(end) = ask.end_lease {
        return answer_end_lease(state, end, &ask.caller.role);
    }
    let handed = match hand_out(&ask.role, &ask.caller.role) {
        HandOut::Granted(handed) => handed,
        HandOut::Refused(why) => return ConnectReply::Refused(refusal_of(why)),
    };
    // A handed-out connection is the shell's, the agent tier's or the portal's, trusted by how it
    // was obtained (the portal's only for the screen and the windows: `Class::of`); its peer stays
    // unproven (the app it belongs to is `ask.caller.app`, kept below).
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
    let owner_of = RemoteFor::Portal(PortalSessionId(remote.session));
    match open_socket(state, owner_of, remote.devices, owner) {
        Ok(client) => ConnectReply::Granted(client),
        Err(why) => ConnectReply::Refused(why),
    }
}

/// Opens a remote input socket for `owner_of` offering the `devices` bit set: its server end
/// becomes an event-loop source feeding the model input tagged with `owner_of`'s origin, with a
/// device slot of its own (CASW-19), asked for by the bus connection `owner`. The client end is
/// returned.
fn open_socket(
    state: &mut State,
    owner_of: RemoteFor,
    devices: u32,
    owner: Option<String>,
) -> Result<std::os::fd::OwnedFd, ConnectRefusal> {
    let Some(slot) = state.agent.remote.mint() else {
        tracing::warn!("every remote input slot has been used");
        return Err(ConnectRefusal::Unavailable);
    };
    let extent = desktop_extent(state);
    let grant = open_remote_eis(owner_of, slot, RemoteKinds::from_wire(devices), extent).map_err(
        |error| {
            tracing::warn!(%error, "a remote input socket could not be made");
            ConnectRefusal::Unavailable
        },
    )?;
    let inserted = state.event_loop.insert_source(
        grant.source,
        move |event, (), state: &mut State| match event {
            RemoteEvent::Input(input) => {
                let at = state.clock.now();
                deliver_remote(state, input.at(at));
            }
            RemoteEvent::Ended => {
                state.agent.remote.closed(slot);
                end_lease_of(state, owner_of);
            }
            _ => {}
        },
    );
    match inserted {
        Ok(token) => {
            state.agent.remote.opened(grant.handle, token, owner);
            Ok(grant.client)
        }
        Err(error) => {
            tracing::warn!(%error, "a remote input source could not be inserted");
            Err(ConnectRefusal::Unavailable)
        }
    }
}

/// Whether the caller holds the agent tier's role: the one caller that may lease a window
/// (the bus thread proved the role; this is the second line).
fn agent_only(caller: &RoleName) -> Result<(), ConnectRefusal> {
    match agent_may_lease(caller) {
        true => Ok(()),
        false => Err(ConnectRefusal::RoleNotAllowed),
    }
}

/// Decides one `LeaseWindow`: only the agent tier may lease, and only a window that is there.
/// The model hears `LeaseGranted` first, so the lease's first event already reaches the
/// window; then the lease's input socket is opened. A socket that could not be made ends the
/// lease again.
pub fn answer_lease(
    state: &mut State,
    ask: LeaseAsk,
    owner: Option<String>,
    caller: &RoleName,
) -> ConnectReply {
    if let Err(why) = agent_only(caller) {
        return ConnectReply::Refused(why);
    }
    let window = window_of_identifier(&ask.window)
        .filter(|window| state.core.model.windows().contains_key(window));
    let Some(window) = window else {
        return ConnectReply::Refused(ConnectRefusal::NoSuchWindow);
    };
    let lease = mint_lease(state);
    feed(state, vec![Event::LeaseGranted { lease, window }]);
    match open_socket(state, RemoteFor::Agent(lease), ask.devices, owner) {
        Ok(socket) => ConnectReply::Leased {
            lease: lease.0,
            socket,
        },
        Err(why) => {
            feed(state, vec![Event::LeaseEnded { lease }]);
            ConnectReply::Refused(why)
        }
    }
}

/// The next lease id, from the compositor's one id mint.
fn mint_lease(state: &mut State) -> LeaseId {
    let mint = std::mem::replace(&mut state.core.mint, casement_core::IdMint::start());
    let (mint, lease) = mint.lease();
    state.core.mint = mint;
    lease
}

/// Decides one `EndLease`: only the agent tier may end a lease. Its socket is taken out of the
/// loop, what it held is let go, and the model forgets the lease. A lease that is not open is
/// not an error.
pub fn answer_end_lease(state: &mut State, end: EndLeaseAsk, caller: &RoleName) -> ConnectReply {
    if let Err(why) = agent_only(caller) {
        return ConnectReply::Refused(why);
    }
    let lease = LeaseId(end.lease);
    let ended = state.agent.remote.end_for(RemoteFor::Agent(lease));
    close(state, ended);
    end_lease_of(state, RemoteFor::Agent(lease));
    ConnectReply::Ended
}

/// The model forgets the lease `owner_of` stands for, if it is one.
fn end_lease_of(state: &mut State, owner_of: RemoteFor) {
    if let RemoteFor::Agent(lease) = owner_of {
        feed(state, vec![Event::LeaseEnded { lease }]);
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
    let ended = state
        .agent
        .remote
        .end_for(RemoteFor::Portal(PortalSessionId(end.session)));
    close(state, ended);
    ConnectReply::Ended
}

/// The portal's or the agent tier's bus connection `gone.owner` left the bus (a restart, a
/// crash): every remote socket and lease it asked for is closed and the keys and buttons held are let go, so an app does not
/// keep injecting input into a session nobody is accounting for any more (NEW-PI-8).
pub fn answer_gone(state: &mut State, gone: &GoneAsk) -> ConnectReply {
    let ended = state.agent.remote.end_owner(&gone.owner);
    close(state, ended);
    ConnectReply::Ended
}

/// Takes `ended` out of the loop and lets go of what each held; a lease among them is
/// forgotten by the model after its releases.
fn close(state: &mut State, ended: Vec<(RemoteHandle, RegistrationToken)>) {
    for (handle, token) in ended {
        state.event_loop.remove(token);
        for release in handle.release() {
            let at = state.clock.now();
            deliver_remote(state, release.at(at));
        }
        end_lease_of(state, handle.owner());
    }
}

/// One input event from a remote session into the model, by the path every input takes. Its
/// origin was set where it entered (`casement-agent`'s EIS) and is not changed here.
pub fn deliver_remote(state: &mut State, event: InputEvent) {
    feed(state, vec![Event::Input(event)]);
}
