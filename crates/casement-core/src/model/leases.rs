//! The agent's leases in the model: granted on a window that is there, ended on request or
//! when the window goes. What a lease lets an event reach is the reach table's
//! ([`crate::input::reach`]).

use super::Model;
use super::moved::Moved;
use crate::ids::{LeaseId, WindowId};

impl Model {
    /// `lease` now holds `window`. A window that is not there is not leased: the lease stays
    /// unknown, so its events reach nothing.
    pub(super) fn grant_lease(self, lease: LeaseId, window: WindowId) -> Moved {
        match self.windows.contains_key(&window) {
            true => Moved::of(Model {
                leases: self.leases.granted(lease, window),
                ..self
            }),
            false => Moved::of(self),
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
