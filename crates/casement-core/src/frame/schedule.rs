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

/// How many vblanks a deadline looks past before giving up and taking the next one (a frame
/// costing more than this many periods cannot be scheduled anyway).
const MAX_SKIPPED: usize = 600;

/// The deadline for the first vblank after `now` that a frame costing `predicted` plus `slack`
/// can still make; a frame that cannot make the next vblank is for the one after.
pub fn next_deadline(
    clock: &VblankClock,
    predicted: Duration,
    now: Mono,
    slack: Duration,
) -> Deadline {
    let need = predicted.saturating_add(slack);
    let first = clock.next_after(now).0;
    let vblank = std::iter::successors(Some(first), |v| Some(clock.next_after(*v).0))
        .take(MAX_SKIPPED)
        .find(|v| v.0.checked_sub(need).is_some_and(|at| at >= now.0))
        .unwrap_or(first);
    let render_at = Mono(vblank.0.saturating_sub(need).max(now.0));
    Deadline { render_at, vblank }
}
