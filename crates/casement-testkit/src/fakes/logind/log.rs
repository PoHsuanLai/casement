//! What the fake saw: the calls in order and the inhibitors still held.

use super::{LogindCall, LogindError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// The shared record, cloned into each served object and the watcher tasks.
#[derive(Debug, Clone, Default)]
pub(super) struct Recorder {
    shared: Arc<Shared>,
}

#[derive(Debug, Default)]
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

#[derive(Debug, Default)]
struct State {
    calls: Vec<LogindCall>,
    held: Vec<Held>,
    next_id: u64,
}

#[derive(Debug)]
struct Held {
    id: u64,
    what: String,
}

impl Recorder {
    fn state(&self) -> MutexGuard<'_, State> {
        self.shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn record(&self, call: LogindCall) {
        self.state().calls.push(call);
    }

    pub(super) fn calls(&self) -> Vec<LogindCall> {
        self.state().calls.clone()
    }

    /// Notes a new inhibitor and returns its id.
    pub(super) fn hold(&self, what: &str) -> u64 {
        let mut state = self.state();
        let id = state.next_id;
        state.next_id += 1;
        state.held.push(Held {
            id,
            what: what.to_owned(),
        });
        state.calls.push(LogindCall::Inhibit(what.to_owned()));
        id
    }

    /// Notes that inhibitor `id`'s pipe closed.
    pub(super) fn release(&self, id: u64) {
        let mut state = self.state();
        if let Some(at) = state.held.iter().position(|held| held.id == id) {
            let held = state.held.remove(at);
            state.calls.push(LogindCall::InhibitReleased(held.what));
        }
        drop(state);
        self.shared.changed.notify_all();
    }

    pub(super) fn held(&self) -> Vec<String> {
        self.state()
            .held
            .iter()
            .map(|held| held.what.clone())
            .collect()
    }

    /// Blocks until no inhibitor is held, for at most `timeout`.
    pub(super) fn wait_released(&self, timeout: Duration) -> Result<(), LogindError> {
        let (state, _timed_out) = self
            .shared
            .changed
            .wait_timeout_while(self.state(), timeout, |state| !state.held.is_empty())
            .unwrap_or_else(PoisonError::into_inner);
        if state.held.is_empty() {
            Ok(())
        } else {
            Err(LogindError::StillHeld {
                what: state.held.iter().map(|held| held.what.clone()).collect(),
            })
        }
    }
}
