//! The software scanout surface (feature `tty`): `ScanoutSurface` over smithay's `DrmCompositor`
//! with a dumb-buffer allocator and pixman, a thin adapter (B7). CASB-07: `casement --safe` has
//! to run on a machine whose GPU driver crashes, so nothing here opens a GBM device, an EGL
//! display or a GLES context.
//!
//! The compositor's swapchain is two or three `XRGB8888` dumb buffers the kernel allocates
//! (`DumbAllocator`), each with its own framebuffer (`DrmDeviceFd` as the exporter). pixman
//! draws into them through their prime mapping (`PixmanRenderer` binds a `Dmabuf`), the damage
//! tracker and the buffer ages are the compositor's, and the flip is the compositor's too
//! (atomic where the device does atomic, else legacy), so a frame is queued and reported
//! presented exactly as on the GLES surface (`pace.rs` counts both). Untested here (no device
//! in the jail): a hardware run checks it (FINDINGS "CASB-07").
//!
//! Every frame is composited by pixman: no overlay plane (a client's buffer is never scanned
//! out directly: a safe session offers no dma-buf), and no hardware cursor plane (the compositor
//! is given no GBM device to allocate cursor buffers on, and the frame flags leave the cursor
//! plane out), so the pointer is drawn into the frame.

use super::kms::{ConnectorId, KmsError};
use super::pace::{Pace, Turn};
use super::surface::{Presented, QueueOutcome, QueuedFrame, ScanoutSurface};
use super::surface_real::{CLEAR, Flips, refused, refused_by};
use crate::headless::transform_of;
use casement_core::output::OutputConfig;
use casement_core::time::Mono;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::dumb::DumbAllocator;
use smithay::backend::allocator::{Format, Fourcc};
use smithay::backend::drm::compositor::{DrmCompositor, FrameFlags};
use smithay::backend::drm::{DrmDeviceFd, DrmSurface};
use smithay::backend::renderer::Bind;
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::output::OutputModeSource;
use smithay::reexports::drm::control::crtc;
use smithay::utils::{Buffer, Scale, Size};
use std::cell::RefCell;
use std::rc::Rc;

/// The compositor type one output's software surface runs: dumb buffers, framebuffers added on
/// the device's own descriptor, no user data per frame, no GBM device for a cursor plane.
type SoftCompositor = DrmCompositor<DumbAllocator, DrmDeviceFd, (), DrmDeviceFd>;

/// The pixman renderer a device's software surfaces share.
pub(super) type SharedPixman = Rc<RefCell<PixmanRenderer>>;

/// What a device lends each software surface it opens.
pub(super) struct SoftShare {
    /// The device's descriptor: the dumb buffers are allocated on it and the framebuffers added.
    pub fd: DrmDeviceFd,
    /// The renderer that draws every output of the device.
    pub renderer: SharedPixman,
    /// The cursor plane's size (only the compositor's bookkeeping; the plane is not used).
    pub cursor: Size<u32, Buffer>,
    /// The device's flip counts.
    pub flips: Rc<RefCell<Flips>>,
}

/// One output's software scanout surface.
pub struct SoftScanout {
    connector: ConnectorId,
    crtc: crtc::Handle,
    compositor: SoftCompositor,
    renderer: SharedPixman,
    flips: Rc<RefCell<Flips>>,
    pace: Pace,
    /// Held so the device knows this connector's surface is out; dropped with the surface.
    _claim: Rc<()>,
}

impl SoftScanout {
    /// A surface for `connector` on `surface`'s CRTC, drawing `config`'s picture with pixman.
    /// `claim` is dropped with the surface.
    pub(super) fn open(
        connector: ConnectorId,
        surface: DrmSurface,
        share: SoftShare,
        claim: Rc<()>,
        config: &OutputConfig,
    ) -> Result<SoftScanout, KmsError> {
        let SoftShare {
            fd,
            renderer,
            cursor,
            flips,
        } = share;
        let crtc = surface.crtc();
        let size = Size::from((
            i32::try_from(config.mode.size.w).unwrap_or(i32::MAX),
            i32::try_from(config.mode.size.h).unwrap_or(i32::MAX),
        ));
        let picture = OutputModeSource::Static {
            size,
            scale: Scale::from(f64::from(config.scale.0) / 120.0),
            transform: transform_of(config.transform),
        };
        let drawable: Vec<Format> = Bind::<Dmabuf>::supported_formats(&*renderer.borrow())
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default();
        let compositor: SoftCompositor = DrmCompositor::new(
            picture,
            surface,
            None,
            DumbAllocator::new(fd.clone()),
            fd,
            [Fourcc::Xrgb8888, Fourcc::Argb8888],
            drawable,
            cursor,
            None,
        )
        .map_err(refused)?;
        Ok(SoftScanout {
            connector,
            crtc,
            compositor,
            renderer,
            flips,
            pace: Pace::new(),
            _claim: claim,
        })
    }

    /// The connector it scans out to.
    pub fn connector(&self) -> ConnectorId {
        self.connector
    }
}

impl ScanoutSurface for SoftScanout {
    fn queue(&mut self, frame: QueuedFrame<'_>) -> QueueOutcome {
        let Turn::Free(seq) = self.pace.turn() else {
            return QueueOutcome::Busy;
        };
        let QueuedFrame::Draw(source) = frame else {
            // Rendered pixels and direct candidates reach the glass through `Draw`.
            return QueueOutcome::Failed(refused("only drawn frames are queued on a device"));
        };
        let rendered = {
            let mut renderer = self.renderer.borrow_mut();
            let elements = source.pixman(&mut renderer);
            self.compositor
                .render_frame(&mut *renderer, &elements, CLEAR, FrameFlags::empty())
                .map(|result| result.is_empty)
                .map_err(refused_by)
        };
        // A frame with no change is not queued: `queue_frame` refuses it as empty.
        let queued = match rendered {
            Ok(true) => return QueueOutcome::Unchanged,
            Ok(false) => self.compositor.queue_frame(()).map_err(refused_by),
            Err(error) => Err(error),
        };
        match queued {
            Ok(()) => {
                self.pace.queued();
                QueueOutcome::Queued(seq)
            }
            Err(error) => QueueOutcome::Failed(error),
        }
    }

    fn presented(&mut self, now: Mono) -> Option<Presented> {
        self.pace.pending()?;
        self.flips.borrow_mut().take(self.crtc)?;
        if let Err(error) = self.compositor.frame_submitted() {
            tracing::warn!(%error, "the compositor did not take a finished flip");
        }
        self.pace.landed(now)
    }

    fn blank(&mut self) -> Result<(), KmsError> {
        self.pace.lost();
        self.flips.borrow_mut().clear(self.crtc);
        self.compositor.clear().map_err(refused)
    }

    fn reset(&mut self) {
        self.pace.lost();
        self.flips.borrow_mut().clear(self.crtc);
        if let Err(error) = self.compositor.reset_state() {
            tracing::warn!(%error, "the compositor did not reset after the seat came back");
        }
    }
}

impl std::fmt::Debug for SoftScanout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoftScanout")
            .field("connector", &self.connector)
            .field("crtc", &self.crtc)
            .field("pace", &self.pace)
            .finish_non_exhaustive()
    }
}
