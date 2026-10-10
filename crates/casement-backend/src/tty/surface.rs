//! The scanout seam (P1-Z; filled by P1-D3): hand one output a finished frame and learn when
//! it reached the glass.

use super::kms::KmsError;
use crate::headless::FramePixels;
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
#[derive(Debug)]
#[non_exhaustive]
pub enum QueuedFrame {
    /// A frame already rendered.
    Rendered(FramePixels),
    /// A client buffer for direct scanout (the surface falls back to compositing it).
    Direct(DirectCandidate),
}

/// What queueing a frame did.
#[derive(Debug)]
#[non_exhaustive]
pub enum QueueOutcome {
    /// Queued; `presented` will report it.
    Queued,
    /// A flip is still pending; try at the next vblank.
    Busy,
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

/// One output's scanout. Real: `surface_real.rs` over the device crates' compositor; fake:
/// `casement_testkit::fakes::FakeScanout`.
pub trait ScanoutSurface: std::fmt::Debug {
    /// Queues `frame` for the next vblank.
    fn queue(&mut self, frame: QueuedFrame) -> QueueOutcome;

    /// The oldest frame that reached the glass since the last call.
    fn presented(&mut self) -> Option<Presented>;
}
