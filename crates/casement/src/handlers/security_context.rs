//! `wp_security_context_manager_v1` (P0-H8): a sandbox engine's listening socket joins the
//! event loop; every client it accepts is inserted as `ConnectionRole::Sandboxed` with the
//! context's facts, through the one door (`clients::insert_client`).

use crate::clients::{self, Room, Standing, accept, insert_client};
use crate::core_loop;
use crate::state::State;
use casement_core::policy::ConnectionRole;
use casement_ipc::dbus::peer::Alive;
use casement_wayland::ClientState;
use casement_wayland::security_context::{Maker, identity_for_clients, on_create_listener};
use porter_core::{CgroupPath, PeerFacts, PeerIdentity, SandboxFacts, identity_of};
use rustix::net::sockopt::socket_peercred;
use smithay::wayland::security_context::{
    SecurityContext, SecurityContextHandler, SecurityContextListenerSource,
};
use std::os::unix::net::UnixStream;

impl SecurityContextHandler for State {
    fn context_created(&mut self, source: SecurityContextListenerSource, context: SecurityContext) {
        let (facts, events) = on_create_listener(&mut self.wl.security_context, &context);
        core_loop::feed(self, events);
        let role = ConnectionRole::Sandboxed(facts.clone());
        // Only the desktop's sandbox launcher can give a context's clients an app, and its own
        // proof caps what the facts may prove (CASW-04); a creator that is gone, or was not
        // inserted by casement, is no launcher and proves nothing.
        let (creator, maker) = self
            .display
            .backend_handle()
            .get_client_data(context.creator_client_id.clone())
            .ok()
            .and_then(|data| {
                (*data)
                    .downcast_ref::<ClientState>()
                    .map(|state| (state.peer.clone(), state.maker))
            })
            .unwrap_or((PeerIdentity::Unproven, Maker::Other));
        let inserted = accept::insert(
            &self.event_loop,
            source,
            accept::ACCEPT_REST,
            move |stream, _, state: &mut State| {
                if clients::room_in_session(state) != Room::Free {
                    return;
                }
                let peer = identity_for_clients(&creator, maker, peer_of(&stream, &facts));
                if clients::room_for_peer(state, &peer, &stream) != Room::Free {
                    return;
                }
                if let Err(error) =
                    insert_client(state, stream, role.clone(), peer, Standing::Unjudged)
                {
                    tracing::warn!(%error, "a security-context client could not be inserted");
                }
            },
        );
        if let Err(error) = inserted {
            tracing::warn!(%error, "a security-context listener could not join the event loop");
        }
    }
}

/// Who the facts say the process on `stream` is, from its credentials, its cgroup and the
/// context's facts. The connector is held by its pidfd before its cgroup is read, and the reading
/// counts only if the process still runs afterwards, so a pid another process took after the
/// connector exited never names the client. What cannot be read leaves the peer unproven.
/// (Whether the context's creator lets the facts count is [`identity_for_clients`]'s.)
fn peer_of(stream: &UnixStream, facts: &SandboxFacts) -> PeerIdentity {
    let Ok(cred) = socket_peercred(stream) else {
        return PeerIdentity::Unproven;
    };
    let pid = cred.pid.as_raw_nonzero().get();
    let Some(held) = clients::handle_of(stream, pid) else {
        return PeerIdentity::Unproven;
    };
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))
        .ok()
        .and_then(|text| CgroupPath::from_proc_cgroup(&text).ok())
        .filter(|_| clients::alive(&held) == Alive::Yes);
    cgroup.map_or(PeerIdentity::Unproven, |cgroup| {
        identity_of(&PeerFacts {
            pid: pid.unsigned_abs(),
            cgroup,
            sandbox: facts.clone(),
        })
    })
}
