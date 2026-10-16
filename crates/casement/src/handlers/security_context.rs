//! `wp_security_context_manager_v1` (P0-H8): a sandbox engine's listening socket joins the
//! event loop; every client it accepts is inserted as `ConnectionRole::Sandboxed` with the
//! context's facts, through the one door (`clients::insert_client`).

use crate::clients::{Standing, insert_client};
use crate::core_loop;
use crate::state::State;
use casement_core::policy::ConnectionRole;
use casement_wayland::ClientState;
use casement_wayland::security_context::{identity_for_clients, on_create_listener};
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
        // The creator's own proof caps what the context's facts may prove (CASW-04); a creator
        // that is gone, or was not inserted by casement, proves nothing.
        let creator = self
            .display
            .backend_handle()
            .get_client_data(context.creator_client_id.clone())
            .ok()
            .and_then(|data| {
                (*data)
                    .downcast_ref::<ClientState>()
                    .map(|state| state.peer.clone())
            })
            .unwrap_or(PeerIdentity::Unproven);
        let inserted = self
            .event_loop
            .insert_source(source, move |stream, _, state| {
                let peer = identity_for_clients(&creator, peer_of(&stream, &facts));
                if let Err(error) =
                    insert_client(state, stream, role.clone(), peer, Standing::Unjudged)
                {
                    tracing::warn!(%error, "a security-context client could not be inserted");
                }
            });
        if let Err(error) = inserted {
            tracing::warn!(%error, "a security-context listener could not join the event loop");
        }
    }
}

/// Who the facts say the process on `stream` is, from its credentials, its cgroup and the
/// context's facts. What cannot be read leaves the peer unproven. (Whether the context's creator
/// lets the facts count is [`identity_for_clients`]'s.)
fn peer_of(stream: &UnixStream, facts: &SandboxFacts) -> PeerIdentity {
    let Ok(cred) = socket_peercred(stream) else {
        return PeerIdentity::Unproven;
    };
    let pid = cred.pid.as_raw_nonzero().get();
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))
        .ok()
        .and_then(|text| CgroupPath::from_proc_cgroup(&text).ok());
    cgroup.map_or(PeerIdentity::Unproven, |cgroup| {
        identity_of(&PeerFacts {
            pid: pid.unsigned_abs(),
            cgroup,
            sandbox: facts.clone(),
        })
    })
}
