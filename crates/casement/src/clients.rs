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

mod processes;
pub use processes::Processes;

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
    if role == ConnectionRole::Public {
        state
            .processes
            .record(key, &stream, state.xwayland.satellite_pid());
    }
    let client = state.display.insert_client(
        stream,
        Arc::new(
            ClientState::new(key, role.clone(), peer.clone())
                .with_gone(state.wl.clients.gone_queue()),
        ),
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

/// The disconnects since the last call, as the events the model takes: the loop feeds them each
/// turn (smithay announces a disconnect from a callback with no access to the state, so
/// `ClientState` queues it: `casement_wayland::client::GoneQueue`).
pub fn take_gone(state: &mut State) -> Vec<Event> {
    let gone = state.wl.clients.gone_queue().take();
    gone.iter()
        .for_each(|client| state.processes.forget(*client));
    gone.into_iter()
        .map(|client| Event::ClientGone { client })
        .collect()
}
