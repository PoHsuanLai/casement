//! What casement keeps per Wayland client: its key, the socket role it came through (I7) and
//! smithay's per-client compositor state.

use casement_core::ClientKey;
use casement_core::policy::ConnectionRole;
use casement_core::window::WindowOrigin;
use porter_core::PeerIdentity;
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, Resource};
use smithay::wayland::compositor::CompositorClientState;
use std::sync::{Arc, Mutex};

/// The key of the client that owns `surface`, when casement inserted that client.
pub fn key_of_surface(surface: &WlSurface) -> Option<ClientKey> {
    surface
        .client()
        .as_ref()
        .and_then(ClientState::of)
        .map(|data| data.key)
}

/// The clients that disconnected since the loop last looked. smithay announces a disconnect
/// from `ClientData::disconnected`, which has no access to the state, so the key is queued
/// here and the loop drains it (`casement::clients::take_gone`) into `Event::ClientGone`.
#[derive(Debug, Clone, Default)]
pub struct GoneQueue(Arc<Mutex<Vec<ClientKey>>>);

impl GoneQueue {
    /// `client` went away.
    fn push(&self, client: ClientKey) {
        self.0
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(client);
    }

    /// The clients that went away since the last call, in the order they went.
    pub fn take(&self) -> Vec<ClientKey> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|poison| poison.into_inner()))
    }
}

impl PartialEq for GoneQueue {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for GoneQueue {}

/// The data every client connection carries from the moment it is inserted into the display.
#[derive(Debug)]
pub struct ClientState {
    /// The client's key in the model.
    pub key: ClientKey,
    /// Which socket it came through; decides which globals it sees (I7).
    pub role: ConnectionRole,
    /// Who its process is, read once where the connection arrived.
    pub peer: PeerIdentity,
    /// Whether every toplevel it maps is an X11 program's (the satellite's connection).
    pub origin: WindowOrigin,
    /// smithay's per-client compositor state (client scale, blocker bookkeeping).
    pub compositor: CompositorClientState,
    /// Where this client's disconnect is reported ([`ClientMint::gone_queue`]).
    gone: GoneQueue,
}

impl ClientState {
    /// The data for a client keyed `key` that arrived through `role`.
    pub fn new(key: ClientKey, role: ConnectionRole, peer: PeerIdentity) -> ClientState {
        ClientState {
            key,
            role,
            peer,
            origin: WindowOrigin::Wayland,
            compositor: CompositorClientState::default(),
            gone: GoneQueue::default(),
        }
    }

    /// The same client, whose toplevels come from `origin`.
    pub fn with_origin(self, origin: WindowOrigin) -> ClientState {
        ClientState { origin, ..self }
    }

    /// The same client reporting its disconnect to `gone`.
    pub fn with_gone(self, gone: GoneQueue) -> ClientState {
        ClientState { gone, ..self }
    }

    /// The state `client` was inserted with, if it was inserted by casement.
    pub fn of(client: &Client) -> Option<&ClientState> {
        client.get_data::<ClientState>()
    }
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {
        self.gone.push(self.key);
    }
}

/// Mints client keys in order from 1; a key is never reused in a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientMint {
    next: u64,
    gone: GoneQueue,
}

impl ClientMint {
    /// A mint whose first key is 1.
    pub fn start() -> ClientMint {
        ClientMint {
            next: 1,
            gone: GoneQueue::default(),
        }
    }

    /// The queue every client minted here reports its disconnect to.
    pub fn gone_queue(&self) -> GoneQueue {
        self.gone.clone()
    }

    /// The next key, and the mint after it.
    pub fn next(self) -> (ClientMint, ClientKey) {
        (
            ClientMint {
                next: self.next.saturating_add(1),
                gone: self.gone,
            },
            ClientKey(self.next),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnects_queue_in_order_and_drain_once() {
        let (mint, first) = ClientMint::start().next();
        let (mint, second) = mint.next();
        let queue = mint.gone_queue();
        queue.push(second);
        queue.push(first);
        assert_eq!(queue.take(), vec![second, first]);
        assert!(queue.take().is_empty());
        assert_eq!(mint.gone_queue(), queue);
    }
}
