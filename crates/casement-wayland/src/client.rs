//! What casement keeps per Wayland client: its key, the socket role it came through (I7) and
//! smithay's per-client compositor state.

use crate::consent::{ImageWatch, Ledger, Unwatched};
use casement_core::ClientKey;
use casement_core::policy::{ConnectionRole, PowerSet};
use casement_core::privilege::{ClientStanding, Privilege};
use casement_core::window::WindowOrigin;
use porter_core::PeerIdentity;
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, Resource};
use smithay::wayland::compositor::CompositorClientState;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// The key of the client that owns `surface`, when casement inserted that client.
pub fn key_of_surface(surface: &WlSurface) -> Option<ClientKey> {
    surface
        .client()
        .as_ref()
        .and_then(ClientState::of)
        .map(|data| data.key)
}

/// Clients to tell that a commit they had held back may go on, each once, in key order
/// (`blocker_cleared`). Kept by key so noting a client costs a lookup, not a scan of the list
/// (CASW-20).
#[derive(Debug, Default)]
pub struct ClearedClients(BTreeMap<ClientKey, Client>);

impl ClearedClients {
    /// Notes `client`; one casement did not insert is not noted (it has no commits of ours).
    pub fn note(&mut self, client: Client) {
        if let Some(key) = ClientState::of(&client).map(|data| data.key) {
            self.0.entry(key).or_insert(client);
        }
    }

    /// The clients noted since the last call, in key order.
    pub fn take(&mut self) -> Vec<Client> {
        std::mem::take(&mut self.0).into_values().collect()
    }
}

/// The clients that disconnected since the loop last looked. smithay announces a disconnect
/// from `ClientData::disconnected`, which has no access to the state, so the key is queued
/// here and the loop drains it (`casement::clients::take_gone`) into `Event::ClientGone`. The
/// queue also holds the consent ledger, so a disconnect forgets what was said of the client's
/// powers at once.
#[derive(Debug, Clone, Default)]
pub struct GoneQueue {
    gone: Arc<Mutex<Vec<ClientKey>>>,
    ledger: Ledger,
}

impl GoneQueue {
    /// A queue that forgets a gone client's rows in `ledger`.
    fn over(ledger: Ledger) -> GoneQueue {
        GoneQueue {
            gone: Arc::default(),
            ledger,
        }
    }

    /// `client` went away.
    fn push(&self, client: ClientKey) {
        self.ledger.forget(client);
        self.gone
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(client);
    }

    /// The clients that went away since the last call, in the order they went.
    pub fn take(&self) -> Vec<ClientKey> {
        std::mem::take(
            &mut *self
                .gone
                .lock()
                .unwrap_or_else(|poison| poison.into_inner()),
        )
    }
}

impl PartialEq for GoneQueue {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.gone, &other.gone)
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
    /// How well the compositor knows its program, as of accept. Unproven until the root says
    /// more ([`ClientState::with_standing`]); the model holds the same word
    /// (`Event::ClientStanding`).
    pub standing: ClientStanding,
    /// The powers whose globals this client is not shown, decided once at accept from the
    /// person's and the administrator's refusals: a refusal that arrives later leaves the
    /// globals advertised and every use gets the protocol's denial.
    pub hidden: PowerSet,
    /// Whether the program still runs the image it had at accept (read at a power's first use).
    pub image: Arc<dyn ImageWatch>,
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
            standing: ClientStanding::unproven(),
            hidden: PowerSet::default(),
            image: Arc::new(Unwatched),
            origin: WindowOrigin::Wayland,
            compositor: CompositorClientState::default(),
            gone: GoneQueue::default(),
        }
    }

    /// The same client, whose toplevels come from `origin`.
    pub fn with_origin(self, origin: WindowOrigin) -> ClientState {
        ClientState { origin, ..self }
    }

    /// The same client, whose program is known as `standing`, shown every global but those of
    /// the `hidden` powers.
    pub fn with_standing(self, standing: ClientStanding, hidden: PowerSet) -> ClientState {
        ClientState {
            standing,
            hidden,
            ..self
        }
    }

    /// The same client, whose program is known as `standing`, shown the globals `privilege`
    /// does not refuse that standing (the person's book and the administrator's floor at this
    /// moment).
    pub fn with_standing_under(
        self,
        privilege: &Privilege,
        standing: ClientStanding,
    ) -> ClientState {
        let hidden = privilege.hidden_for(&self.role, &standing);
        self.with_standing(standing, hidden)
    }

    /// The same client, whose image `image` watches.
    pub fn with_image_watch(self, image: Arc<dyn ImageWatch>) -> ClientState {
        ClientState { image, ..self }
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
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {
        self.gone.push(self.key);
    }
}

/// Mints client keys in order from 1; a key is never reused in a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientMint {
    next: u64,
    gone: GoneQueue,
    ledger: Ledger,
}

impl ClientMint {
    /// A mint whose first key is 1.
    pub fn start() -> ClientMint {
        let ledger = Ledger::default();
        ClientMint {
            next: 1,
            gone: GoneQueue::over(ledger.clone()),
            ledger,
        }
    }

    /// The consent ledger: the latest word per client and power, shared with every client
    /// minted here.
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
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
                ledger: self.ledger,
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
