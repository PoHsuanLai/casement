//! The agent's leases in the model: granted on a window that is there, ended on request or
//! when the window goes. What a lease lets an event reach is the reach table's
//! ([`crate::input::reach`]).

use super::Model;
use super::moved::Moved;
use crate::ids::{LeaseId, WindowId};
use casement_protocols::vocab::WindowRole;

/// Whether a window may be leased to the agent tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Leasable {
    /// It may.
    Yes,
    /// No window has that id.
    NoSuchWindow,
    /// The shell's own window (sill's menus, sheets and panels, or any window of a client that
    /// came through the shell's connection): the agent never drives the desktop's own surfaces.
    Shell,
}

impl Model {
    /// Whether `window` may be leased: it is there and it is not the shell's.
    pub fn leasable(&self, window: WindowId) -> Leasable {
        match self.windows.get(&window) {
            None => Leasable::NoSuchWindow,
            Some(held) if held.role == WindowRole::Shell || self.is_shell(held.client) => {
                Leasable::Shell
            }
            Some(_) => Leasable::Yes,
        }
    }

    /// `lease` now holds `window`. A window that may not be leased ([`Model::leasable`]) is
    /// not: the lease stays unknown, so its events reach nothing.
    pub(super) fn grant_lease(self, lease: LeaseId, window: WindowId) -> Moved {
        match self.leasable(window) {
            Leasable::Yes => Moved::of(Model {
                leases: self.leases.granted(lease, window),
                ..self
            }),
            Leasable::NoSuchWindow | Leasable::Shell => Moved::of(self),
        }
    }

    /// `lease` ended; its events reach nothing from now on.
    pub(super) fn end_lease(self, lease: LeaseId) -> Moved {
        Moved::of(Model {
            leases: self.leases.ended(lease),
            ..self
        })
    }

    /// The leases with none left on `window`, which went away.
    pub(super) fn drop_leases_of(self, window: WindowId) -> Model {
        Model {
            leases: self.leases.without_window(window),
            ..self
        }
    }
}
