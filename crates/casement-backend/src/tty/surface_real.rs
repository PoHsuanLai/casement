//! The real scanout surface (feature `tty`): `ScanoutSurface` over smithay's `DrmCompositor`,
//! a thin adapter (B7). Untested here (no device in the jail): a hardware run checks it
//! (FINDINGS "p1-d3" and "p1-t").
//!
//! The compositor chooses planes itself: the primary plane takes the frame, the cursor plane
//! takes the pointer when `casement_core::frame::cursor_plane_for` says it fits (the caller
//! hands the compositor the cursor element last, and it either promotes it or composites it),
//! and overlay planes take direct-scanout candidates when the driver accepts the test commit.
//!
//! Each surface draws with the GLES renderer of its own device's GBM node (`casement_render::
//! gles_on`), one renderer shared by every output of the device (two EGL displays on one native
//! display would terminate each other). One render GPU for every device (the copy across GPUs
//! the soft topology tests prove) is not wired: a second GPU's outputs are drawn by that GPU.
//! The device's vblank events reach the surface through [`Flips`], which the device's event
//! source fills.

use super::kms::{ConnectorId, KmsError};
use super::surface::{Presented, QueueOutcome, QueuedFrame, ScanoutSurface};
use crate::headless::transform_of;
use casement_core::output::OutputConfig;
use casement_core::time::{Mono, VblankSeq};
use smithay::backend::allocator::Fourcc;
use smithay::backend::allocator::gbm::{GbmAllocator, GbmBufferFlags};
use smithay::backend::drm::compositor::{DrmCompositor, FrameFlags};
use smithay::backend::drm::exporter::gbm::{GbmFramebufferExporter, NodeFilter};
use smithay::backend::drm::{DrmDeviceFd, DrmSurface};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::output::OutputModeSource;
use smithay::reexports::drm::control::crtc;
use smithay::reexports::gbm::Device as GbmDevice;
use smithay::utils::{Buffer, Size};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Black, opaque: what shows where no element draws.
const CLEAR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// The compositor type one output's surface runs.
type Compositor =
    DrmCompositor<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, (), DrmDeviceFd>;

/// The flips the device reported done, by CRTC: its event source adds, the surface takes.
#[derive(Debug, Default)]
pub struct Flips {
    done: HashMap<crtc::Handle, u32>,
}

impl Flips {
    /// The device reported a flip done on `crtc`.
    pub fn done(&mut self, crtc: crtc::Handle) {
        let count = self.done.entry(crtc).or_insert(0);
        *count = count.saturating_add(1);
    }

    /// Takes one flip done on `crtc`, if there is one.
    fn take(&mut self, crtc: crtc::Handle) -> Option<()> {
        let count = self.done.get_mut(&crtc).filter(|count| **count > 0)?;
        *count -= 1;
        Some(())
    }
}

/// One output's scanout surface.
pub struct RealScanout {
    connector: ConnectorId,
    crtc: crtc::Handle,
    compositor: Compositor,
    renderer: Rc<RefCell<GlesRenderer>>,
    flips: Rc<RefCell<Flips>>,
    /// The vblank the next queued frame is for (counted here: the kernel's own sequence is
    /// not used, so a missed vblank is not measured on real hardware; FINDINGS "p1-t").
    next: VblankSeq,
    /// The vblank of the frame whose flip is in flight.
    pending: Option<VblankSeq>,
    /// Held so the device knows this connector's surface is out; dropped with the surface.
    _claim: Rc<()>,
}

/// What a device lends each surface it opens: its buffers, its renderer, its cursor plane size
/// and its flip counts.
pub(super) struct DeviceShare<'a> {
    /// The device's buffer allocator.
    pub gbm: &'a GbmDevice<DrmDeviceFd>,
    /// The device's one renderer.
    pub renderer: Rc<RefCell<GlesRenderer>>,
    /// The cursor plane's size.
    pub cursor: Size<u32, Buffer>,
    /// The device's flip counts.
    pub flips: Rc<RefCell<Flips>>,
}

impl RealScanout {
    /// A surface for `connector` on `surface`'s CRTC, drawing `config`'s picture with the
    /// device's renderer. `claim` is dropped with the surface.
    pub(super) fn open(
        connector: ConnectorId,
        surface: DrmSurface,
        share: DeviceShare<'_>,
        claim: Rc<()>,
        config: &OutputConfig,
    ) -> Result<RealScanout, KmsError> {
        let DeviceShare {
            gbm,
            renderer,
            cursor,
            flips,
        } = share;
        let crtc = surface.crtc();
        let allocator = GbmAllocator::new(
            gbm.clone(),
            GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
        );
        let exporter = GbmFramebufferExporter::new(gbm.clone(), NodeFilter::All);
        let render_formats: Vec<_> = renderer
            .borrow()
            .egl_context()
            .dmabuf_render_formats()
            .iter()
            .copied()
            .collect();
        let size = Size::from((
            i32::try_from(config.mode.size.w).unwrap_or(i32::MAX),
            i32::try_from(config.mode.size.h).unwrap_or(i32::MAX),
        ));
        let picture = OutputModeSource::Static {
            size,
            scale: smithay::utils::Scale::from(f64::from(config.scale.0) / 120.0),
            transform: transform_of(config.transform),
        };
        let compositor = DrmCompositor::new(
            picture,
            surface,
            None,
            allocator,
            exporter,
            [Fourcc::Xrgb8888, Fourcc::Argb8888],
            render_formats,
            cursor,
            Some(gbm.clone()),
        )
        .map_err(refused)?;
        Ok(RealScanout {
            connector,
            crtc,
            compositor,
            renderer,
            flips,
            next: VblankSeq(0),
            pending: None,
            _claim: claim,
        })
    }

    /// The connector it scans out to.
    pub fn connector(&self) -> ConnectorId {
        self.connector
    }
}

impl ScanoutSurface for RealScanout {
    fn queue(&mut self, frame: QueuedFrame<'_>) -> QueueOutcome {
        if self.pending.is_some() {
            return QueueOutcome::Busy;
        }
        let QueuedFrame::Draw(source) = frame else {
            // Rendered pixels and direct candidates reach the glass through `Draw`: the
            // compositor does its own plane assignment (see the module doc).
            return QueueOutcome::Failed(refused("only drawn frames are queued on a device"));
        };
        let mut renderer = self.renderer.borrow_mut();
        let elements = source.gles(&mut renderer);
        let rendered = self
            .compositor
            .render_frame(&mut *renderer, &elements, CLEAR, FrameFlags::DEFAULT)
            .map(|result| result.is_empty)
            .map_err(refused);
        // A frame with no change is not queued: `queue_frame` refuses it as empty.
        let queued = match rendered {
            Ok(true) => return QueueOutcome::Unchanged,
            Ok(false) => self.compositor.queue_frame(()).map_err(refused),
            Err(error) => Err(error),
        };
        match queued {
            Ok(()) => {
                self.pending = Some(self.next);
                QueueOutcome::Queued(self.next)
            }
            Err(error) => QueueOutcome::Failed(error),
        }
    }

    fn presented(&mut self, now: Mono) -> Option<Presented> {
        let seq = self.pending?;
        self.flips.borrow_mut().take(self.crtc)?;
        if let Err(error) = self.compositor.frame_submitted() {
            tracing::warn!(%error, "the compositor did not take a finished flip");
        }
        self.pending = None;
        self.next = VblankSeq(seq.0.saturating_add(1));
        Some(Presented::new(seq, now))
    }

    fn blank(&mut self) -> Result<(), KmsError> {
        self.pending = None;
        self.compositor.clear().map_err(refused)
    }

    fn reset(&mut self) {
        self.pending = None;
        if let Err(error) = self.compositor.reset_state() {
            tracing::warn!(%error, "the compositor did not reset after the seat came back");
        }
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
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}
