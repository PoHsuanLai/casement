//! Client connections: every client enters through here, with its role (I7) and its
//! identity, whether it came through the public socket, a security-context listener, a
//! privileged hand-out (P2) or a test kit socket pair.

use crate::core_loop;
use crate::state::State;
use casement_core::Event;
use casement_core::policy::ConnectionRole;
use casement_wayland::ClientState;
use porter_core::PeerIdentity;
use smithay::reexports::wayland_server::Client;
use std::os::unix::net::UnixStream;
use std::sync::Arc;

/// Inserts the client on `stream` with `role` and `peer`, mints its key and tells the model
/// (`Event::ClientConnected`).
pub fn insert_client(
    state: &mut State,
    stream: UnixStream,
    role: ConnectionRole,
    peer: PeerIdentity,
) -> std::io::Result<Client> {
    let (clients, key) = state.wl.clients.clone().next();
    state.wl.clients = clients;
    let client = state.display.insert_client(
        stream,
        Arc::new(ClientState::new(key, role.clone(), peer.clone())),
    )?;
    core_loop::feed(
        state,
        vec![Event::ClientConnected {
            client: key,
            peer,
            role,
        }],
    );
    Ok(client)
}
