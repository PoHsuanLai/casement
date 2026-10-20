//! What each agent lease pressed and the seat delivered, still down. The release of an agent
//! press is delivered only when its press was (`core_loop::agent_gate`): a press the gate or the
//! reach table held back is not here, so its release reaches no one, and the person's own presses
//! and releases (`Taken`) are never touched by a lease's.

use casement_core::LeaseId;
use casement_core::input::{ButtonCode, KeyCode};
use std::collections::BTreeSet;

/// A key or a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pressed {
    /// A key.
    Key(KeyCode),
    /// A pointer button.
    Button(ButtonCode),
}

/// The agent presses delivered and not yet released, by lease.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentDown(BTreeSet<(LeaseId, Pressed)>);

impl AgentDown {
    /// Nothing down.
    pub fn none() -> AgentDown {
        AgentDown::default()
    }

    /// `lease` pressed `what` and it is on its way to a client.
    #[must_use]
    pub fn pressed(mut self, lease: LeaseId, what: Pressed) -> AgentDown {
        self.0.insert((lease, what));
        self
    }

    /// `lease`'s press of `what` was held back after all: its release is not delivered.
    #[must_use]
    pub fn held_back(mut self, lease: LeaseId, what: Pressed) -> AgentDown {
        self.0.remove(&(lease, what));
        self
    }

    /// Whether `lease`'s press of `what` was delivered and is still down.
    pub fn holds(&self, lease: LeaseId, what: Pressed) -> bool {
        self.0.contains(&(lease, what))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_is_down_for_its_lease_alone_until_it_is_held_back() {
        let key = Pressed::Key(KeyCode(30));
        let down = AgentDown::none().pressed(LeaseId(1), key);
        assert!(down.holds(LeaseId(1), key));
        assert!(!down.holds(LeaseId(2), key));
        assert!(!down.holds(LeaseId(1), Pressed::Button(ButtonCode(0x110))));
        assert!(!down.held_back(LeaseId(1), key).holds(LeaseId(1), key));
    }
}
