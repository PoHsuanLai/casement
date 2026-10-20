//! A lease keys its window: an assistant run about to send keys asks that the window it holds
//! be the key window. The lease is the person's request carried out, so it has its own path
//! into focus ([`FocusIn::LeaseFocus`]) and rule A7, which refuses apps, stays exact.

use super::Model;
use super::moved::Moved;
use crate::focus::FocusIn;
use crate::ids::{LeaseId, WindowId};
use crate::lock::LockState;
use crate::window::{SpacePlace, WindowState};

/// Why a lease may not take the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LeaseFocusRefusal {
    /// The lease is not open (it ended, or its window went).
    NoLease,
    /// The screen is locked or locking: nothing moves focus until it is unlocked.
    Locked,
    /// The window is in the dock; only the person brings it back.
    Minimised,
}

impl Model {
    /// The window `lease` may key now, or why it may not. A lease exists only for a window of
    /// an app the agent tier was allowed to drive, so a live lease is the grant.
    pub fn lease_focus(&self, lease: LeaseId) -> Result<WindowId, LeaseFocusRefusal> {
        if !matches!(self.lock, LockState::Unlocked) {
            return Err(LeaseFocusRefusal::Locked);
        }
        let window = self
            .leases
            .window_of(lease)
            .ok_or(LeaseFocusRefusal::NoLease)?;
        match self.windows.get(&window).map(|held| &held.state) {
            None => Err(LeaseFocusRefusal::NoLease),
            Some(WindowState::Minimised { .. }) => Err(LeaseFocusRefusal::Minimised),
            Some(_) => Ok(window),
        }
    }

    /// `lease`'s window comes forward and takes the key: its app is shown again if hidden and
    /// its Space is shown (rule S5) before focus keys it. Nothing moves for a refused lease.
    pub(super) fn focus_leased(self, lease: LeaseId) -> Moved {
        let Ok(window) = self.lease_focus(lease) else {
            return Moved::of(self);
        };
        let Some(held) = self.windows.get(&window) else {
            return Moved::of(self);
        };
        let app = held.app.clone();
        let space = match &held.place {
            SpacePlace::On(id) => Some(id.clone()),
            SpacePlace::AllSpaces => None,
        };
        self.unhide_app(&app)
            .then(|model| match space {
                Some(id) => model.show(&id),
                None => Moved::of(model),
            })
            .then(|model| model.feed(FocusIn::LeaseFocus { window }))
    }
}
