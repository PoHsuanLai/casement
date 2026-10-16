//! Clients: which app each serves, and what goes when one disconnects.

use super::Model;
use super::moved::Moved;
use crate::ids::ClientKey;
use casement_protocols::vocab::Toggle;

impl Model {
    /// A client connected; it serves no app until it maps a window.
    pub(super) fn connect(self, client: ClientKey) -> Moved {
        let mut model = self;
        model.clients.entry(client).or_insert(None);
        Moved::of(model)
    }

    /// A client now holds (or let go of) a manager bound at the version that carries `quit`.
    /// One that is not connected is not recorded.
    pub(super) fn quit_listener(self, client: ClientKey, toggle: Toggle) -> Moved {
        let mut model = self;
        match (toggle, model.clients.contains_key(&client)) {
            (Toggle::On, true) => {
                model.quit_listeners.insert(client);
            }
            (Toggle::On, false) | (Toggle::Off, _) => {
                model.quit_listeners.remove(&client);
            }
        }
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
            model.quit_listeners.remove(&client);
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
