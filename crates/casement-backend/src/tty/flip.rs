//! Page-flip bookkeeping (P1-Z; filled by P1-D3): which vblank each queued frame was made for
//! and how it turned out.

use super::surface::Presented;
use casement_core::frame::FrameOutcome;
use casement_core::time::VblankSeq;

/// One output's pending flip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlipTracker {
    /// The vblank the queued frame was made for, while one is pending.
    pub wanted: Option<VblankSeq>,
}

impl FlipTracker {
    /// A tracker with no flip pending.
    pub fn new() -> FlipTracker {
        FlipTracker::default()
    }

    /// A frame for `wanted` was queued.
    #[must_use]
    pub fn queued(self, wanted: VblankSeq) -> FlipTracker {
        let _ = (self, wanted);
        todo!("P1-D3: FlipTracker::queued")
    }

    /// The flip finished: what became of the frame, and the tracker with nothing pending.
    pub fn presented(self, presented: Presented) -> (FlipTracker, FrameOutcome) {
        let _ = (self, presented);
        todo!("P1-D3: FlipTracker::presented")
    }
}
