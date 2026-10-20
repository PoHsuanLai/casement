//! The agent's leases: which window each lease holds. An event of [`super::Origin::Agent`]
//! reaches that window's surfaces and popups and nothing else ([`super::reach`]).

use crate::ids::{LeaseId, WindowId};
use std::collections::BTreeMap;

/// The leases granted and not yet ended, each holding one window.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Leases(BTreeMap<LeaseId, WindowId>);

impl Leases {
    /// No lease.
    pub fn none() -> Leases {
        Leases(BTreeMap::new())
    }

    /// The window `lease` holds; `None` for a lease that ended or was never granted.
    pub fn window_of(&self, lease: LeaseId) -> Option<WindowId> {
        self.0.get(&lease).copied()
    }

    /// The same leases with `lease` holding `window`.
    #[must_use]
    pub fn granted(mut self, lease: LeaseId, window: WindowId) -> Leases {
        self.0.insert(lease, window);
        self
    }

    /// The same leases without `lease`.
    #[must_use]
    pub fn ended(mut self, lease: LeaseId) -> Leases {
        self.0.remove(&lease);
        self
    }

    /// The same leases without any that holds `window` (it closed).
    #[must_use]
    pub fn without_window(mut self, window: WindowId) -> Leases {
        self.0.retain(|_, held| *held != window);
        self
    }

    /// Every lease and its window, in lease order.
    pub fn iter(&self) -> impl Iterator<Item = (LeaseId, WindowId)> + '_ {
        self.0.iter().map(|(lease, window)| (*lease, *window))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lease_holds_its_window_until_it_ends_or_the_window_goes() {
        let leases = Leases::none()
            .granted(LeaseId(1), WindowId(7))
            .granted(LeaseId(2), WindowId(8));
        assert_eq!(leases.window_of(LeaseId(1)), Some(WindowId(7)));
        assert_eq!(leases.window_of(LeaseId(3)), None);
        let leases = leases.ended(LeaseId(1));
        assert_eq!(leases.window_of(LeaseId(1)), None);
        let leases = leases.without_window(WindowId(8));
        assert_eq!(leases.iter().count(), 0);
    }
}
