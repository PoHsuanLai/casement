//! The consent gate of a virtual keyboard (`Power::Keys`): a keyboard made while the answer is
//! out keeps its requests, at most [`HELD_REQUESTS`], and replays them in order when the person
//! allows; a refused keyboard is dead (its requests go nowhere, nothing reaches a client), and
//! so is one whose power is taken back (the keys it held down are released at once).

use casement_core::privilege::HELD_REQUESTS;
use smithay::reexports::wayland_protocols_misc::zwp_virtual_keyboard_v1::server::zwp_virtual_keyboard_v1::Request;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Where a keyboard stands with the person.
#[derive(Debug)]
enum State {
    /// Allowed: requests are carried out.
    Open,
    /// The answer is out: requests wait here.
    Waiting(Vec<Request>),
    /// Refused or taken back: requests are ignored.
    Dead,
}

/// What became of a request that reached a keyboard.
#[derive(Debug)]
pub(super) enum Routed {
    /// Carry it out now.
    Now(Request),
    /// It waits for the answer.
    Kept,
    /// It goes nowhere.
    Dropped,
}

/// Whether a keyboard is made already allowed, waiting or refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Start {
    /// The power is allowed.
    Open,
    /// The answer is out.
    Waiting,
    /// The power is refused.
    Dead,
}

/// One keyboard's gate.
#[derive(Debug)]
pub(super) struct Gate(Mutex<State>);

impl Gate {
    /// The gate of a keyboard made in `start`.
    pub(super) fn new(start: Start) -> Gate {
        Gate(Mutex::new(match start {
            Start::Open => State::Open,
            Start::Waiting => State::Waiting(Vec::new()),
            Start::Dead => State::Dead,
        }))
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Where `request` goes.
    pub(super) fn route(&self, request: Request) -> Routed {
        match &mut *self.state() {
            State::Open => Routed::Now(request),
            State::Waiting(held) if held.len() < HELD_REQUESTS => {
                held.push(request);
                Routed::Kept
            }
            State::Waiting(_) | State::Dead => Routed::Dropped,
        }
    }

    /// The person allowed: the gate opens, and the requests it kept are returned in order. A
    /// refused keyboard stays dead.
    pub(super) fn open(&self) -> Vec<Request> {
        let mut state = self.state();
        match std::mem::replace(&mut *state, State::Open) {
            State::Waiting(held) => held,
            State::Open => Vec::new(),
            State::Dead => {
                *state = State::Dead;
                Vec::new()
            }
        }
    }

    /// The power is refused or taken back: the gate closes for good and drops what it kept.
    pub(super) fn close(&self) {
        *self.state() = State::Dead;
    }

    /// How many requests wait.
    #[cfg(test)]
    pub(super) fn kept(&self) -> usize {
        match &*self.state() {
            State::Waiting(held) => held.len(),
            State::Open | State::Dead => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> Request {
        Request::Key {
            time: 0,
            key: 30,
            state: 1,
        }
    }

    #[test]
    fn a_waiting_keyboard_keeps_requests_up_to_the_cap_and_replays_them_on_allow() {
        let gate = Gate::new(Start::Waiting);
        for _ in 0..HELD_REQUESTS {
            assert!(matches!(gate.route(key()), Routed::Kept));
        }
        assert!(matches!(gate.route(key()), Routed::Dropped));
        assert_eq!(gate.kept(), HELD_REQUESTS);
        assert_eq!(gate.open().len(), HELD_REQUESTS);
        assert!(matches!(gate.route(key()), Routed::Now(_)));
    }

    #[test]
    fn a_refused_keyboard_stays_dead_even_when_told_to_open() {
        let gate = Gate::new(Start::Dead);
        assert!(matches!(gate.route(key()), Routed::Dropped));
        assert!(gate.open().is_empty());
        assert!(matches!(gate.route(key()), Routed::Dropped));
        let open = Gate::new(Start::Open);
        open.close();
        assert!(matches!(open.route(key()), Routed::Dropped));
    }
}
