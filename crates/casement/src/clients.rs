//! Client connections: every client enters through here, with its role (I7), its identity and
//! how well its program is known, whether it came through the public socket, a
//! security-context listener, a privileged hand-out (P2) or a test kit socket pair.

use crate::core_loop;
use crate::state::State;
use casement_core::Event;
use casement_core::policy::ConnectionRole;
use casement_core::privilege::ClientStanding;
use casement_wayland::ClientState;
use casement_wayland::consent::ImageWatch;
use porter_core::PeerIdentity;
use smithay::reexports::wayland_server::Client;
use std::os::unix::net::UnixStream;
use std::sync::Arc;

pub mod accept;
#[cfg(any(feature = "testing", feature = "debug-inject"))]
mod dev_callers;
mod image;
mod judge;
mod probes;
mod processes;
mod roster;
mod standing;
pub use judge::{Arrival, Judge, desktop_table};
pub use probes::begin as begin_probe;
pub use probes::{MAX_PROBES, PROBE_DEADLINE, Probes};
pub use processes::Processes;
pub use roster::{Holder, MAX_CONNECTIONS, MAX_PER_HOLDER, RESERVED_FOR_PROVEN, Room, Roster};
pub use standing::{client_standing, peer_of};

/// How well the program behind a connection is known when it arrives.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Standing {
    /// Not judged: a connection handed out to the shell or the agent tier (trusted by how it
    /// was obtained), a sandbox's, or one nobody looked at.
    Unjudged,
    /// A program on the public socket, judged against the desktop's caller table.
    Judged {
        /// What it was taken to be at accept.
        standing: ClientStanding,
        /// Whether it still runs the image it had then.
        watch: Arc<dyn ImageWatch>,
    },
}

/// Inserts the client on `stream` with `role`, `peer` and `standing`, mints its key and tells
/// the model (`Event::ClientConnected`, then `Event::ClientStanding` for a judged one).
pub fn insert_client(
    state: &mut State,
    stream: UnixStream,
    role: ConnectionRole,
    peer: PeerIdentity,
    standing: Standing,
) -> std::io::Result<Client> {
    let holder = Holder::of_stream(&peer, &stream);
    let (clients, key) = state.wl.clients.clone().next();
    state.wl.clients = clients;
    if role == ConnectionRole::Public {
        state
            .processes
            .record(key, &stream, state.xwayland.satellite_pid());
    }
    let data = ClientState::new(key, role.clone(), peer.clone())
        // The satellite's connection is recognised by now: its windows are X11 ones.
        .with_origin(state.processes.origin_of(key))
        .with_gone(state.wl.clients.gone_queue());
    let (data, judged) = match standing {
        Standing::Unjudged => (data, None),
        Standing::Judged { standing, watch } => (
            data.with_standing_under(state.core.model.privilege(), standing.clone())
                .with_image_watch(watch),
            Some(standing),
        ),
    };
    let client = state.display.insert_client(stream, Arc::new(data))?;
    // Recorded here, not in `ClientData::initialized`: the system libwayland backend never calls it.
    state.wl.clients.ledger().connected(key, client.id());
    state.roster.join(key, holder);
    let connected = Event::ClientConnected {
        client: key,
        peer,
        role,
    };
    let events = std::iter::once(connected)
        .chain(judged.map(|standing| Event::ClientStanding {
            client: key,
            standing,
        }))
        .collect();
    core_loop::feed(state, events);
    Ok(client)
}

/// The disconnects since the last call, as the events the model takes: the loop feeds them each
/// turn (smithay announces a disconnect from a callback with no access to the state, so
/// `ClientState` queues it: `casement_wayland::client::GoneQueue`).
pub fn take_gone(state: &mut State) -> Vec<Event> {
    let gone = state.wl.clients.gone_queue().take();
    gone.iter().for_each(|client| {
        state.processes.forget(*client);
        state.roster.leave(*client);
    });
    gone.into_iter()
        .map(|client| Event::ClientGone { client })
        .collect()
}

/// Whether the session takes one more connection on a public or sandbox socket, before anything
/// is read from the program behind it. A refusal is logged at each doubling of the count, so a
/// flood does not fill the journal. The caller drops the stream when there is no room.
pub fn room_in_session(state: &mut State) -> Room {
    let room = state.roster.room();
    note_refusal(state, room);
    room
}

/// Whether the session takes one more connection from the program on `stream`, whose identity
/// is `peer`: each app (or process) holds a few, not the session's whole share.
pub fn room_for_peer(state: &mut State, peer: &PeerIdentity, stream: &UnixStream) -> Room {
    let room = state.roster.room_for(&Holder::of_stream(peer, stream));
    note_refusal(state, room);
    room
}

/// Counts and, at each doubling, logs a refused connection.
fn note_refusal(state: &mut State, room: Room) {
    if room == Room::Free {
        return;
    }
    let refused = state.roster.refused();
    if refused.is_power_of_two() {
        tracing::warn!(
            ?room,
            refused,
            live = state.roster.len(),
            "connections are being refused: the session or one program holds as many as it may"
        );
    }
}
