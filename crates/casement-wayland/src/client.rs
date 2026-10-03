//! What casement keeps per Wayland client: its key, the socket role it came through (I7) and
//! smithay's per-client compositor state.

use casement_core::ClientKey;
use casement_core::policy::ConnectionRole;
use porter_core::PeerIdentity;
use smithay::reexports::wayland_server::Client;
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::wayland::compositor::CompositorClientState;

/// The data every client connection carries from the moment it is inserted into the display.
#[derive(Debug)]
pub struct ClientState {
    /// The client's key in the model.
    pub key: ClientKey,
    /// Which socket it came through; decides which globals it sees (I7).
    pub role: ConnectionRole,
    /// Who its process is, read once where the connection arrived.
    pub peer: PeerIdentity,
    /// smithay's per-client compositor state (client scale, blocker bookkeeping).
    pub compositor: CompositorClientState,
}

impl ClientState {
    /// The data for a client keyed `key` that arrived through `role`.
    pub fn new(key: ClientKey, role: ConnectionRole, peer: PeerIdentity) -> ClientState {
        ClientState {
            key,
            role,
            peer,
            compositor: CompositorClientState::default(),
        }
    }

    /// The state `client` was inserted with, if it was inserted by casement.
    pub fn of(client: &Client) -> Option<&ClientState> {
        client.get_data::<ClientState>()
    }
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

/// Mints client keys in order from 1; a key is never reused in a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientMint {
    next: u64,
}

impl ClientMint {
    /// A mint whose first key is 1.
    pub fn start() -> ClientMint {
        ClientMint { next: 1 }
    }

    /// The next key, and the mint after it.
    pub fn next(self) -> (ClientMint, ClientKey) {
        (
            ClientMint {
                next: self.next.saturating_add(1),
            },
            ClientKey(self.next),
        )
    }
}
