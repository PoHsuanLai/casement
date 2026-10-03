//! `wp_commit_timing_v1`: a commit carries the instant it should be presented at; the clock is
//! the session's `Mono` (virtual in tests).

use crate::states::{GlobalFilter, WaylandFault};
use crate::presentation::Presented;
use casement_core::{Event, OutputKey};
use smithay::reexports::wayland_protocols::wp::commit_timing::v1::server::wp_commit_timing_manager_v1::WpCommitTimingManagerV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::commit_timing::{CommitTimingManagerData, CommitTimingManagerState};

/// `wp_commit_timing_manager_v1`.
#[derive(Debug)]
pub struct CommitTimingPart {
    /// smithay's state.
    pub state: CommitTimingManagerState,
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
        }
    }
}

/// A committed surface set a timestamp barrier.
pub fn on_barrier(part: &mut CommitTimingPart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H1: commit_timing::on_barrier")
}

/// `output` is about to present at `presented.at`: barriers due by then are signalled.
pub fn apply_presented(
    part: &mut CommitTimingPart,
    output: &OutputKey,
    presented: Presented,
) -> Result<(), WaylandFault> {
    let _ = (part, output, presented);
    todo!("P0-H1: commit_timing::apply_presented")
}
