//! Clients: which app each serves, and what goes when one disconnects.

use super::Model;
use super::moved::Moved;
use crate::ids::ClientKey;

impl Model {
    /// A client connected; it serves no app until it maps a window.
    pub(super) fn connect(self, client: ClientKey) -> Moved {
        let mut model = self;
        model.clients.entry(client).or_insert(None);
        Moved::of(model)
    }

    /// A client went: its windows unmap, and its app goes with its last client.
    pub(super) fn disconnect(self, client: ClientKey) -> Moved {
        let owned: Vec<_> = self
            .windows
            .values()
            .filter(|window| window.client == client)
            .map(|window| window.id)
            .collect();
        let moved = owned
            .into_iter()
            .fold(Moved::of(self), |moved, id| moved.then(|m| m.unmap(id)));
        moved.then(|mut model| {
            let served = model.clients.remove(&client).flatten();
            let orphaned =
                served.filter(|app| !model.clients.values().any(|c| c.as_ref() == Some(app)));
            match orphaned {
                Some(app) => model.drop_app(&app),
                None => Moved::of(model),
            }
        })
    }
}
