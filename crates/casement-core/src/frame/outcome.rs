//! Whether a frame made its vblank.

use crate::time::VblankSeq;

/// What became of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrameOutcome {
    /// Presented at the vblank it was made for.
    OnTime,
    /// Presented late, by this many vblanks.
    Missed {
        /// How many vblanks late.
        by: u32,
    },
}

/// The outcome of a frame made for `wanted` and presented at `presented` (a frame shown early
/// is on time).
///
/// ```no_run
/// use casement_core::frame::{FrameOutcome, judge};
/// use casement_core::time::VblankSeq;
///
/// assert_eq!(judge(VblankSeq(5), VblankSeq(6)), FrameOutcome::Missed { by: 1 });
/// ```
pub fn judge(wanted: VblankSeq, presented: VblankSeq) -> FrameOutcome {
    let late = presented.0.saturating_sub(wanted.0);
    match late {
        0 => FrameOutcome::OnTime,
        n => FrameOutcome::Missed {
            by: u32::try_from(n).unwrap_or(u32::MAX),
        },
    }
}
