//! Requests kept while the person decides: at most [`HELD_REQUESTS`] per client, so a client
//! cannot grow the compositor by asking.

use casement_core::ClientKey;
use casement_core::privilege::HELD_REQUESTS;
use std::collections::BTreeMap;

/// Something held that can tell its client is gone (a wire object that died).
pub trait Mortal {
    /// Whether the thing still has a client to answer.
    fn alive(&self) -> bool;
}

/// Whether a request was kept.
#[derive(Debug)]
pub enum Pushed<T> {
    /// It waits.
    Kept,
    /// The client already has [`HELD_REQUESTS`] waiting: the request comes back, to be refused.
    Full(T),
}

/// The held requests of every client, oldest first.
#[derive(Debug)]
pub struct Held<T> {
    by_client: BTreeMap<ClientKey, Vec<T>>,
}

impl<T> Default for Held<T> {
    fn default() -> Held<T> {
        Held {
            by_client: BTreeMap::new(),
        }
    }
}

impl<T: Mortal> Held<T> {
    /// Keeps `item` for `client` unless it already has [`HELD_REQUESTS`] waiting. Items whose
    /// client is gone are dropped first (nobody will resolve them), so a client that connects,
    /// asks and leaves, over and over, cannot grow the table.
    pub fn push(&mut self, client: ClientKey, item: T) -> Pushed<T> {
        self.by_client.retain(|_, items| {
            items.retain(Mortal::alive);
            !items.is_empty()
        });
        let items = self.by_client.entry(client).or_default();
        match items.len() < HELD_REQUESTS {
            true => {
                items.push(item);
                Pushed::Kept
            }
            false => Pushed::Full(item),
        }
    }
}

impl<T> Held<T> {
    /// Takes `client`'s held requests, oldest first.
    pub fn take(&mut self, client: ClientKey) -> Vec<T> {
        self.by_client.remove(&client).unwrap_or_default()
    }

    /// What `client` has waiting, oldest first.
    pub fn of(&self, client: ClientKey) -> &[T] {
        self.by_client.get(&client).map_or(&[], Vec::as_slice)
    }

    /// How many requests `client` has waiting.
    pub fn count(&self, client: ClientKey) -> usize {
        self.by_client.get(&client).map_or(0, Vec::len)
    }

    /// Whether nothing is held.
    pub fn is_empty(&self) -> bool {
        self.by_client.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Req(u32, bool);

    impl Mortal for Req {
        fn alive(&self) -> bool {
            self.1
        }
    }

    #[test]
    fn a_client_holds_at_most_the_cap_and_the_rest_are_dropped() {
        let mut held = Held::default();
        for n in 0..HELD_REQUESTS {
            let pushed = held.push(ClientKey(1), Req(n as u32, true));
            assert!(matches!(pushed, Pushed::Kept));
        }
        assert!(matches!(
            held.push(ClientKey(1), Req(0, true)),
            Pushed::Full(_)
        ));
        assert!(matches!(
            held.push(ClientKey(2), Req(0, true)),
            Pushed::Kept
        ));
        assert_eq!(held.count(ClientKey(1)), HELD_REQUESTS);
        let taken = held.take(ClientKey(1));
        assert_eq!(taken.first().map(|req| req.0), Some(0));
        assert_eq!(held.count(ClientKey(1)), 0);
    }

    #[test]
    fn the_requests_of_a_gone_client_are_dropped_at_the_next_push() {
        let mut held = Held::default();
        held.push(ClientKey(1), Req(1, false));
        held.push(ClientKey(2), Req(2, true));
        assert_eq!(held.count(ClientKey(1)), 0);
        assert_eq!(held.count(ClientKey(2)), 1);
    }
}
