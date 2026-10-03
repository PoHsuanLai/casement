//! `wp_commit_timer_v1` (P0-H1).

use crate::conn::TestClient;
use crate::surface::subsurface::{H1Data, Surface, quiet};
use std::time::Duration;
use wayland_protocols::wp::commit_timing::v1::client::{
    wp_commit_timer_v1::WpCommitTimerV1, wp_commit_timing_manager_v1::WpCommitTimingManagerV1,
};

/// A bound `wp_commit_timing_manager_v1`.
#[derive(Debug)]
pub struct CommitTiming {
    /// The manager.
    pub manager: WpCommitTimingManagerV1,
}

impl CommitTiming {
    /// Binds the global (version 1).
    pub fn bind(client: &TestClient) -> CommitTiming {
        CommitTiming {
            manager: client
                .bind(1..=1, H1Data)
                .expect("the compositor advertises wp_commit_timing_manager_v1"),
        }
    }

    /// The timer object of `surface`.
    pub fn for_surface(&self, client: &TestClient, surface: &Surface) -> WpCommitTimerV1 {
        self.manager
            .get_timer(&surface.surface, &client.qh(), H1Data)
    }
}

/// Sets `timer`'s timestamp for the next commit to `at` on `CLOCK_MONOTONIC` (the session's
/// `Mono`, virtual in the fixture).
pub fn set_timestamp(timer: &WpCommitTimerV1, at: Duration) {
    let secs = at.as_secs();
    timer.set_timestamp(
        u32::try_from(secs >> 32).unwrap_or(u32::MAX),
        u32::try_from(secs & 0xffff_ffff).unwrap_or(u32::MAX),
        at.subsec_nanos(),
    );
}

quiet!(WpCommitTimingManagerV1, WpCommitTimerV1);
