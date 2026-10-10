//! When to start rendering so a frame is ready for the next vblank.

use crate::time::{Mono, VblankClock};
use std::time::Duration;

/// When to render and which vblank the frame is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Deadline {
    /// The instant to start rendering.
    pub render_at: Mono,
    /// The vblank the frame is meant for.
    pub vblank: Mono,
}

impl Deadline {
    /// A deadline.
    pub fn new(render_at: Mono, vblank: Mono) -> Deadline {
        Deadline { render_at, vblank }
    }
}

/// The deadline for the first vblank after `now` that a frame costing `predicted` plus `slack`
/// can still make; a frame that cannot make the next vblank is for the one after.
pub fn next_deadline(
    clock: &VblankClock,
    predicted: Duration,
    now: Mono,
    slack: Duration,
) -> Deadline {
    let _ = (clock, predicted, now, slack);
    todo!("P1-D3: next_deadline")
}
