//! A key of an agent lease that finds the keyboard on a surface outside the lease (the shell's
//! confirm sheet that has just taken the person's answer, the companion panel) waits a short,
//! bounded time instead of being lost: the keyboard usually comes back to the leased window a
//! moment later. It is delivered, in order, once the gate lets it through; if the keyboard has
//! not come back within [`HOLD`] it is dropped, and the drop is logged with its rule
//! ([`Rule::HoldExpired`], at debug level, never the key itself).
//!
//! Nothing is ever delivered to another client, and the focus is never moved: the gate decides
//! each time, exactly as it does for a key that arrives. Keys of one lease keep their order, so
//! a key (press or release) behind a waiting one waits too, and a release whose press was dropped
//! is shut by the gate (`Rule::NoPress`) like any release without a press. Pointer events and
//! the person's input never wait.
//!
//! The queue is drained at the end of every feed and of every loop turn (a client's request,
//! such as a sheet that closes, moves the keyboard outside a feed) and when the hold timer fires
//! ([`TimerKey::AgentHold`]).

use super::agent_gate::{self, Gate, Rule};
use super::deliver::{pointer_at, through};
use crate::exec;
use crate::state::State;
use casement_core::geometry::LogicalPoint;
use casement_core::input::{InputEvent, InputKind};
use casement_core::time::{Due, Mono, TimerKey};
use casement_core::{Effect, LeaseId};
use std::collections::VecDeque;
use std::time::Duration;

/// How long a key waits for the keyboard to come back to the leased window.
pub(super) const HOLD: Duration = Duration::from_millis(500);

/// One key that waits, and the instant it may wait until.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Held {
    lease: LeaseId,
    event: InputEvent,
    until: Mono,
}

/// The keys waiting, oldest first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Waiting(VecDeque<Held>);

impl Waiting {
    fn has(&self, lease: LeaseId) -> bool {
        self.0.iter().any(|held| held.lease == lease)
    }
}

/// An agent event reaches the gate: it goes on, is dropped (with its rule logged) or, for a key,
/// waits.
pub(super) fn arrive(state: &mut State, lease: LeaseId, event: &InputEvent) {
    let key = matches!(event.kind, InputKind::Key { .. });
    if key && state.agent.waiting.has(lease) {
        return hold(state, lease, event);
    }
    match agent_gate::gate(state, lease, event, pointer(state)) {
        Gate::Open => through(state, event),
        Gate::Shut(rule) => log_shut(lease, rule),
        Gate::Wait(rule) => {
            tracing::debug!(lease = lease.0, ?rule, "agent key waits for the keyboard");
            hold(state, lease, event);
        }
    }
}

/// Queues `event` behind the keys already waiting, then lets the queue go as far as it can.
fn hold(state: &mut State, lease: LeaseId, event: &InputEvent) {
    let until = state.clock.now().after(HOLD);
    state.agent.waiting.0.push_back(Held {
        lease,
        event: event.clone(),
        until,
    });
    settle(state);
}

/// Lets the waiting keys go that the gate now lets through, drops those whose time is up (or
/// that the gate now shuts), and asks for a timer at the instant the oldest of the rest must go.
pub(super) fn settle(state: &mut State) {
    let held = std::mem::take(&mut state.agent.waiting.0);
    if held.is_empty() {
        return;
    }
    let now = state.clock.now();
    let at = pointer(state);
    let mut kept = VecDeque::new();
    let mut blocked: Vec<LeaseId> = Vec::new();
    for key in held {
        if blocked.contains(&key.lease) {
            kept.push_back(key);
            continue;
        }
        match agent_gate::gate(state, key.lease, &key.event, at) {
            Gate::Open => through(state, &key.event),
            Gate::Shut(rule) => log_shut(key.lease, rule),
            Gate::Wait(_) if now >= key.until => log_shut(key.lease, Rule::HoldExpired),
            Gate::Wait(_) => {
                blocked.push(key.lease);
                kept.push_back(key);
            }
        }
    }
    let next = kept.iter().map(|key| key.until).min();
    state.agent.waiting.0 = kept;
    if let Some(at) = next {
        let due = Due {
            at,
            key: TimerKey::AgentHold,
        };
        exec::exec(state, vec![Effect::ArmTimer(due)]);
    }
}

fn pointer(state: &State) -> Option<LogicalPoint> {
    pointer_at(&state.wl.seat.seat).map(|(x, y)| LogicalPoint { x, y })
}

fn log_shut(lease: LeaseId, rule: Rule) {
    tracing::debug!(lease = lease.0, ?rule, "agent input shut");
}
