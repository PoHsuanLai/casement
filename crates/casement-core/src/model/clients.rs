//! Clients: which app each serves, and what goes when one disconnects.

use super::Model;
use super::moved::Moved;
use super::privilege::Input;
use crate::ids::ClientKey;
use crate::policy::ConnectionRole;
use casement_protocols::vocab::Toggle;

impl Model {
    /// A client connected through a socket of `role`; it serves no app until it maps a window.
    pub(super) fn connect(self, client: ClientKey, role: &ConnectionRole) -> Moved {
        let mut model = self;
        model.clients.entry(client).or_default();
        if matches!(role, ConnectionRole::Shell) {
            model.shell_clients.insert(client);
        }
        model.privilege_connected(client, role)
    }

    /// Whether `client` came through the shell's own connection.
    pub(super) fn is_shell(&self, client: ClientKey) -> bool {
        self.shell_clients.contains(&client)
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

    /// A client went: its windows unmap, and each app it served goes with that app's last client
    /// (CASC-04: a connection that carried several apps leaves none of them behind).
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
            let was_shell = model.shell_clients.remove(&client);
            let served = model.clients.remove(&client).unwrap_or_default();
            let orphaned: Vec<_> = served
                .into_iter()
                .filter(|app| !model.clients.values().any(|others| others.contains(app)))
                .collect();
            let moved = orphaned.into_iter().fold(Moved::of(model), |moved, app| {
                moved.then(|model| model.drop_app(&app))
            });
            moved
                .then(|model| model.privileged(Input::Gone { client }))
                .then(|model| match (was_shell, model.shell_clients.is_empty()) {
                    (true, true) => model.privileged(Input::ShellGone),
                    (true, false) | (false, _) => Moved::of(model),
                })
        })
    }
}
