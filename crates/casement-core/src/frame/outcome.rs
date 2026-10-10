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

/// The outcome of a frame made for `wanted` and presented at `presented`.
pub fn judge(wanted: VblankSeq, presented: VblankSeq) -> FrameOutcome {
    let _ = (wanted, presented);
    todo!("P1-D3: judge")
}
