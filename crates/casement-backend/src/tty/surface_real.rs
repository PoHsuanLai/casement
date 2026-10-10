//! The real scanout surface (feature `tty`): `ScanoutSurface` over smithay's `DrmCompositor`,
//! a thin adapter (B7). Untested here (no device in the jail): a hardware run checks it
//! (FINDINGS "p1-d3").
//!
//! The compositor chooses planes itself: the primary plane takes the frame, the cursor plane
//! takes the pointer when `casement_core::frame::cursor_plane_for` says it fits (the caller
//! hands the compositor the cursor element last, and it either promotes it or composites it),
//! and overlay planes take direct-scanout candidates when the driver accepts the test commit.

use super::kms::{ConnectorId, CrtcId, KmsError};
use super::surface::{Presented, QueueOutcome, QueuedFrame, ScanoutSurface};
use casement_core::time::{Mono, VblankSeq};
use smithay::backend::allocator::gbm::GbmAllocator;
use smithay::backend::drm::DrmDeviceFd;
use smithay::backend::drm::compositor::{DrmCompositor, FrameFlags};
use smithay::backend::drm::exporter::gbm::GbmFramebufferExporter;
use smithay::backend::renderer::gles::GlesRenderer;
use std::collections::VecDeque;

/// Black, opaque: what shows where no element draws.
const CLEAR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// The compositor type one output's surface runs.
type Compositor =
    DrmCompositor<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, (), DrmDeviceFd>;

/// One output's scanout surface.
pub struct RealScanout {
    connector: ConnectorId,
    crtc: CrtcId,
    compositor: Compositor,
    renderer: GlesRenderer,
    /// The vblank the next queued frame is for.
    next: VblankSeq,
    /// Flips the device completed, not yet taken.
    done: VecDeque<Presented>,
}

impl RealScanout {
    /// A surface for `connector` on `crtc` drawing with `renderer` through `compositor`.
    pub fn new(
        connector: ConnectorId,
        crtc: CrtcId,
        compositor: Compositor,
        renderer: GlesRenderer,
    ) -> RealScanout {
        RealScanout {
            connector,
            crtc,
            compositor,
            renderer,
            next: VblankSeq(0),
            done: VecDeque::new(),
        }
    }

    /// The connector it scans out to.
    pub fn connector(&self) -> ConnectorId {
        self.connector
    }

    /// The CRTC that drives it.
    pub fn crtc(&self) -> CrtcId {
        self.crtc
    }

    /// The device reported the flip done at vblank `seq` of instant `at` (the DRM event
    /// source calls this from the loop).
    pub fn flip_done(&mut self, seq: VblankSeq, at: Mono) -> Result<(), KmsError> {
        self.compositor.frame_submitted().map_err(refused)?;
        self.done.push_back(Presented::new(seq, at));
        self.next = VblankSeq(seq.0.saturating_add(1));
        Ok(())
    }
}

impl ScanoutSurface for RealScanout {
    fn queue(&mut self, frame: QueuedFrame<'_>) -> QueueOutcome {
        let QueuedFrame::Draw(source) = frame else {
            // Rendered pixels and direct candidates reach the glass through `Draw`: the
            // compositor does its own plane assignment (see the module doc).
            return QueueOutcome::Failed(refused("only drawn frames are queued on a device"));
        };
        let elements = source.gles(&mut self.renderer);
        let queued = self
            .compositor
            .render_frame(&mut self.renderer, &elements, CLEAR, FrameFlags::DEFAULT)
            .map_err(refused)
            .and_then(|_| self.compositor.queue_frame(()).map_err(refused));
        match queued {
            Ok(()) => QueueOutcome::Queued(self.next),
            Err(error) => QueueOutcome::Failed(error),
        }
    }

    fn presented(&mut self) -> Option<Presented> {
        self.done.pop_front()
    }
}

/// A compositor failure as the seam's error (the typed sources stay in the log).
fn refused(error: impl std::fmt::Display) -> KmsError {
    KmsError::Io(std::io::Error::other(error.to_string()))
}

impl std::fmt::Debug for RealScanout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealScanout")
            .field("connector", &self.connector)
            .field("crtc", &self.crtc)
            .field("next", &self.next)
            .finish_non_exhaustive()
    }
}
