//! Page-flip bookkeeping (P1-Z; filled by P1-D3): which vblank each queued frame was made for
//! and how it turned out.

use super::surface::Presented;
use casement_core::frame::{FrameOutcome, judge};
use casement_core::time::VblankSeq;

/// One output's pending flip.
///
/// ```no_run
/// use casement_backend::tty::{FlipTracker, Presented};
/// use casement_core::frame::FrameOutcome;
/// use casement_core::time::{Mono, VblankSeq};
/// use std::time::Duration;
///
/// let tracker = FlipTracker::new().queued(VblankSeq(4));
/// let (tracker, outcome) = tracker.presented(Presented::new(VblankSeq(5), Mono(Duration::ZERO)));
/// assert_eq!(outcome, FrameOutcome::Missed { by: 1 });
/// assert_eq!(tracker.wanted, None);
/// ```
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
        FlipTracker {
            wanted: Some(wanted),
        }
    }

    /// The flip finished: what became of the frame, and the tracker with nothing pending. A
    /// flip nobody queued (a modeset's first frame) is on time by definition.
    pub fn presented(self, presented: Presented) -> (FlipTracker, FrameOutcome) {
        let wanted = self.wanted.unwrap_or(presented.seq);
        (FlipTracker::new(), judge(wanted, presented.seq))
    }
}
