//! `wp_commit_timing_v1`: a commit carries the instant it should be presented at; the clock is
//! the session's `Mono` (virtual in tests).

use crate::presentation::Presented;
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::{Event, OutputKey};
use smithay::reexports::wayland_protocols::wp::commit_timing::v1::server::wp_commit_timing_manager_v1::WpCommitTimingManagerV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, DisplayHandle, GlobalDispatch, Resource, Weak};
use smithay::utils::{Monotonic, Time};
use smithay::wayland::commit_timing::{
    CommitTimerBarrierStateUserData, CommitTimerStateUserData, CommitTimingManagerData,
    CommitTimingManagerState, Timestamp,
};
use smithay::wayland::compositor::with_states;

/// `wp_commit_timing_manager_v1`.
#[derive(Debug)]
pub struct CommitTimingPart {
    /// smithay's state.
    pub state: CommitTimingManagerState,
    /// Surfaces that carry commit timers with barriers still to signal.
    watched: Vec<Weak<WlSurface>>,
    /// Clients whose blocked commits a signalled barrier may have released
    /// ([`take_cleared`]).
    cleared: Vec<Client>,
}

impl CommitTimingPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> CommitTimingPart
    where
        D: GlobalDispatch<WpCommitTimingManagerV1, CommitTimingManagerData> + 'static,
    {
        let _ = role_filter;
        CommitTimingPart {
            state: CommitTimingManagerState::new::<D>(display),
            watched: Vec::new(),
            cleared: Vec::new(),
        }
    }
}

/// Whether `surface` has a timestamp set or a barrier still waiting (smithay registers the
/// barrier in its own pre-commit hook, so this reads true on either side of it).
fn is_timed(surface: &WlSurface) -> bool {
    with_states(surface, |states| {
        let stamped = states
            .data_map
            .get::<CommitTimerStateUserData>()
            .is_some_and(|state| state.borrow().timestamp.is_some());
        let waiting = states
            .data_map
            .get::<CommitTimerBarrierStateUserData>()
            .is_some_and(|barriers| {
                barriers
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .next_deadline()
                    .is_some()
            });
        stamped || waiting
    })
}

/// A committing surface set a timestamp barrier. Commits that carry a timestamp are held back
/// by smithay before `CompositorHandler::commit` runs, so the handler calls this from a
/// pre-commit hook, and the surface is watched until its barriers are signalled.
pub fn on_barrier(part: &mut CommitTimingPart, surface: &WlSurface) -> Vec<Event> {
    let known = part
        .watched
        .iter()
        .any(|watched| *watched == surface.downgrade());
    if !known && is_timed(surface) {
        part.watched.push(surface.downgrade());
    }
    Vec::new()
}

/// `output` is about to present at `presented.at`: barriers due by then are signalled (the
/// timestamps are `CLOCK_MONOTONIC`, which the session's `Mono` is, virtual in tests). P0
/// presents each surface on the first output (A28), so any output's frame counts. The clients
/// that had commits held back are listed for [`take_cleared`].
pub fn apply_presented(
    part: &mut CommitTimingPart,
    output: &OutputKey,
    presented: Presented,
) -> Result<(), WaylandFault> {
    let _ = output;
    let deadline = Timestamp::from(Time::<Monotonic>::from(presented.at.0));
    let mut still = Vec::new();
    for watched in std::mem::take(&mut part.watched) {
        let Ok(surface) = watched.upgrade() else {
            continue;
        };
        let (signalled, remaining) = with_states(&surface, |states| {
            states
                .data_map
                .get::<CommitTimerBarrierStateUserData>()
                .map(|barriers| {
                    let mut barriers = barriers.lock().unwrap_or_else(|poison| poison.into_inner());
                    (
                        barriers.signal_until(deadline),
                        barriers.next_deadline().is_some(),
                    )
                })
                .unwrap_or((false, false))
        });
        if signalled
            && let Some(client) = surface.client()
            && !part.cleared.iter().any(|known| known.id() == client.id())
        {
            part.cleared.push(client);
        }
        if remaining {
            still.push(watched);
        }
    }
    part.watched = still;
    Ok(())
}

/// The clients whose held-back commits a barrier signalled since the last call may have
/// released: the caller tells each one's smithay compositor state (`blocker_cleared`).
pub fn take_cleared(part: &mut CommitTimingPart) -> Vec<Client> {
    std::mem::take(&mut part.cleared)
}

/// `surface` was destroyed: it is no longer watched.
pub fn forget_surface(part: &mut CommitTimingPart, surface: &WlSurface) {
    part.watched
        .retain(|watched| *watched != surface.downgrade());
}
