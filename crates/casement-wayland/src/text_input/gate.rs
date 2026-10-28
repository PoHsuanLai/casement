//! The activation gate: the first key after a text field is enabled must reach the input
//! method, not the document (SP1 A5, A16).
//!
//! The protocol's order is: the field sends `enable` and `commit`, the compositor tells the
//! input method `activate`, the input method answers by grabbing the keyboard. A key that
//! arrives between `enable` and the grab would reach the document raw. While a method is
//! bound, a field is active and the keyboard is not yet grabbed, keys wait here, in order,
//! for the grab or for [`HOLD_LIMIT`] (a method that never grabs must not stall typing).
//! The gate sees only facts (read from smithay's handles) and instants (arguments), so it is
//! a pure state machine.

use crate::seat::KeyDelivery;
use casement_core::time::Mono;
use std::time::Duration;

/// How long a key waits for the input method's grab, counted from when the gate first saw
/// the activation.
pub const HOLD_LIMIT: Duration = Duration::from_millis(50);

/// What the seat's text-input and input-method handles say right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImeFacts {
    /// Whether an input method is bound to the seat (none: no fcitx5).
    pub method_bound: bool,
    /// Whether a text field is enabled on the focused client.
    pub field_active: bool,
    /// Whether the input method holds the keyboard grab (keys reach it through smithay's grab).
    pub grabbed: bool,
}

impl ImeFacts {
    /// Whether the input method is expected to grab the keyboard soon: bound, a field active,
    /// no grab yet.
    fn expects_grab(self) -> bool {
        self.method_bound && self.field_active && !self.grabbed
    }
}

/// Where the gate is in one activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// No activation in progress.
    Idle,
    /// An activation was first seen at this instant.
    Waiting { since: Mono },
    /// The input method did not grab in time; keys go on to the document until the field
    /// ends.
    GaveUp,
}

/// The gate and the keys waiting at it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyGate {
    phase: Phase,
    held: Vec<KeyDelivery>,
}

impl Default for KeyGate {
    fn default() -> KeyGate {
        KeyGate::new()
    }
}

impl KeyGate {
    /// An idle gate holding nothing.
    pub fn new() -> KeyGate {
        KeyGate {
            phase: Phase::Idle,
            held: Vec::new(),
        }
    }

    /// `key` arrived at `now`: the keys to deliver now, in order (empty while it waits).
    pub fn admit(&mut self, facts: ImeFacts, key: KeyDelivery, now: Mono) -> Vec<KeyDelivery> {
        self.held.push(key);
        self.settle(facts, now)
    }

    /// Time or the facts moved (call after every client dispatch round and when the hold
    /// timer fires): the waiting keys that may go now, in order.
    pub fn settle(&mut self, facts: ImeFacts, now: Mono) -> Vec<KeyDelivery> {
        self.phase = next_phase(self.phase, facts, now);
        match self.phase {
            Phase::Waiting { .. } => Vec::new(),
            Phase::Idle | Phase::GaveUp => std::mem::take(&mut self.held),
        }
    }

    /// A new activation begins: the wait for the grab starts afresh (waiting keys stay).
    pub fn restart(&mut self) {
        self.phase = Phase::Idle;
    }

    /// When the oldest wait ends, if a key is waiting: arm a timer for it.
    pub fn due(&self) -> Option<Mono> {
        match (self.phase, self.held.is_empty()) {
            (Phase::Waiting { since }, false) => Some(since.after(HOLD_LIMIT)),
            _ => None,
        }
    }
}

/// The phase after seeing `facts` at `now`.
fn next_phase(phase: Phase, facts: ImeFacts, now: Mono) -> Phase {
    match (facts.expects_grab(), phase) {
        (false, _) => Phase::Idle,
        (true, Phase::Idle) => Phase::Waiting { since: now },
        (true, Phase::Waiting { since }) if now >= since.after(HOLD_LIMIT) => Phase::GaveUp,
        (true, kept) => kept,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::input::{KeyCode, Origin};
    use casement_core::{DeviceId, InputSerial};
    use casement_protocols::vocab::PressEdge;

    fn at(ms: u64) -> Mono {
        Mono(Duration::from_millis(ms))
    }

    fn key(code: u32, ms: u64) -> KeyDelivery {
        KeyDelivery {
            code: KeyCode(code),
            edge: PressEdge::Press,
            serial: InputSerial(code),
            time: at(ms),
            origin: Origin::Test,
            device: DeviceId(1),
        }
    }

    const ENABLED: ImeFacts = ImeFacts {
        method_bound: true,
        field_active: true,
        grabbed: false,
    };
    const GRABBED: ImeFacts = ImeFacts {
        grabbed: true,
        ..ENABLED
    };
    const NO_METHOD: ImeFacts = ImeFacts {
        method_bound: false,
        ..ENABLED
    };
    const NO_FIELD: ImeFacts = ImeFacts {
        field_active: false,
        ..ENABLED
    };

    #[test]
    fn without_a_method_or_a_field_keys_go_straight_through() {
        for facts in [NO_METHOD, NO_FIELD] {
            let mut gate = KeyGate::new();
            assert_eq!(gate.admit(facts, key(30, 0), at(0)), vec![key(30, 0)]);
            assert_eq!(gate.due(), None);
        }
    }

    #[test]
    fn the_first_key_after_enable_waits_for_the_input_methods_grab() {
        let mut gate = KeyGate::new();
        assert!(gate.settle(ENABLED, at(100)).is_empty());
        assert!(gate.admit(ENABLED, key(30, 101), at(101)).is_empty());
        assert!(gate.admit(ENABLED, key(31, 102), at(102)).is_empty());
        assert_eq!(gate.due(), Some(at(150)));
        // The grab arrives: both go, in order, ahead of anything newer.
        assert_eq!(
            gate.settle(GRABBED, at(103)),
            vec![key(30, 101), key(31, 102)]
        );
        assert_eq!(gate.due(), None);
        assert_eq!(
            gate.admit(GRABBED, key(32, 104), at(104)),
            vec![key(32, 104)]
        );
    }

    #[test]
    fn a_method_that_never_grabs_is_given_the_hold_limit_once_per_field() {
        let mut gate = KeyGate::new();
        assert!(gate.admit(ENABLED, key(30, 0), at(0)).is_empty());
        assert_eq!(gate.settle(ENABLED, at(49)), vec![]);
        assert_eq!(gate.settle(ENABLED, at(50)), vec![key(30, 0)]);
        // The gate gave up on this field: later keys are not delayed.
        assert_eq!(gate.admit(ENABLED, key(31, 60), at(60)), vec![key(31, 60)]);
        // A new field starts a new wait.
        assert_eq!(gate.settle(NO_FIELD, at(70)), vec![]);
        assert!(gate.admit(ENABLED, key(32, 80), at(80)).is_empty());
    }

    #[test]
    fn a_field_that_ends_while_keys_wait_lets_them_through() {
        let mut gate = KeyGate::new();
        assert!(gate.admit(ENABLED, key(30, 0), at(0)).is_empty());
        assert_eq!(gate.settle(NO_FIELD, at(10)), vec![key(30, 0)]);
    }
}
