//! The scanout seam (P1-Z; filled by P1-D3): hand one output a finished frame and learn when
//! it reached the glass.

use super::kms::KmsError;
use crate::headless::{FramePixels, FrameSource};
use casement_core::WindowId;
use casement_core::time::{Mono, VblankSeq};

/// A client buffer a plane could scan out directly instead of compositing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DirectCandidate {
    /// The window whose buffer it is.
    pub window: WindowId,
}

impl DirectCandidate {
    /// A candidate.
    pub fn new(window: WindowId) -> DirectCandidate {
        DirectCandidate { window }
    }
}

/// What to put on the glass.
#[non_exhaustive]
pub enum QueuedFrame<'a> {
    /// A frame already rendered.
    Rendered(FramePixels),
    /// A scene the surface draws itself, with the renderer its device uses.
    Draw(&'a dyn FrameSource),
    /// A client buffer for direct scanout (the surface falls back to compositing it).
    Direct(DirectCandidate),
}

impl std::fmt::Debug for QueuedFrame<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueuedFrame::Rendered(pixels) => f.debug_tuple("Rendered").field(pixels).finish(),
            QueuedFrame::Draw(_) => f.write_str("Draw(..)"),
            QueuedFrame::Direct(candidate) => f.debug_tuple("Direct").field(candidate).finish(),
        }
    }
}

/// What queueing a frame did.
#[derive(Debug)]
#[non_exhaustive]
pub enum QueueOutcome {
    /// Queued for this vblank; `presented` will report it.
    Queued(VblankSeq),
    /// A flip is still pending; try at the next vblank.
    Busy,
    /// The frame changed nothing on the glass, so none was queued (not a failure).
    Unchanged,
    /// The device refused.
    Failed(KmsError),
}

/// A frame that reached the glass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Presented {
    /// Its vblank.
    pub seq: VblankSeq,
    /// When it happened, on the session clock.
    pub at: Mono,
}

impl Presented {
    /// A presented frame.
    pub fn new(seq: VblankSeq, at: Mono) -> Presented {
        Presented { seq, at }
    }
}

/// One output's scanout. Real: `surface_real.rs` over the device crates' compositor with a GPU, or
/// `soft_real.rs` over dumb buffers and pixman when no GPU is used (`render_mode.rs`); fake:
/// `casement_testkit::fakes::FakeScanout`.
pub trait ScanoutSurface: std::fmt::Debug {
    /// Queues `frame` for the next vblank.
    fn queue(&mut self, frame: QueuedFrame<'_>) -> QueueOutcome;

    /// The oldest frame that reached the glass since the last call. `now` is the session
    /// clock's instant of this look: a device that learns of a flip without a time on the
    /// session clock stamps it so.
    fn presented(&mut self, now: Mono) -> Option<Presented>;

    /// The output goes dark (DPMS off): stop scanning out. The next frame queued lights it
    /// again. Nothing to do by default.
    fn blank(&mut self) -> Result<(), KmsError> {
        Ok(())
    }

    /// The seat came back (a VT switch back, a resume): whatever the surface remembered of the
    /// screen is stale, so the next frame is a full one and a flip that was in flight is lost.
    fn reset(&mut self) {}
}
