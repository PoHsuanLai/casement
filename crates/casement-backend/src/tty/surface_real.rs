//! The real scanout surface (feature `tty`): `ScanoutSurface` over smithay's `DrmCompositor`,
//! a thin adapter (B7). Untested here (no device in the jail): a hardware run checks it
//! (FINDINGS "p1-d3" and "p1-t").
//!
//! The compositor chooses planes itself: the primary plane takes the frame, the cursor plane
//! takes the pointer when `casement_core::frame::cursor_plane_for` says it fits (the caller
//! hands the compositor the cursor element last, and it either promotes it or composites it),
//! and overlay planes take direct-scanout candidates when the driver accepts the test commit.
//!
//! A surface draws one of two ways, by the render plan's route for its device. `Direct`: the
//! GLES renderer of the GPU that renders and scans out (`casement_render::gles_on`, one renderer
//! per GPU, shared by every output: two EGL displays on one native display would terminate each
//! other) builds the elements and the compositor draws them. `Copy`: the render GPU's renderer
//! draws the picture into a buffer the scanout GPU allocated (`bridge_real.rs`) and the
//! compositor, running on the scanout GPU's own renderer, puts that one texture on the glass.
//! When a copy route cannot be built the surface says so in the log and draws as `Direct` on the
//! scanout GPU's renderer. The device's vblank events reach the surface through [`Flips`], which
//! the device's event source fills.

use super::bridge_real::{Bridge, Drawn};
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
pub(super) const CLEAR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// The compositor type one output's surface runs.
pub(super) type Compositor =
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

/// A GLES renderer other code lends, shared by every output of its GPU.
pub(super) type Shared = Rc<RefCell<GlesRenderer>>;

/// Which renderers draw a device's outputs.
pub(super) enum Feed {
    /// The device's own renderer draws and the device scans out (the render GPU, or a machine
    /// with one GPU).
    Own(Shared),
    /// The render GPU draws and this device's renderer composes the copy.
    Copy {
        /// The render GPU's renderer.
        render: Shared,
        /// This device's renderer.
        local: Shared,
    },
}

/// How one surface draws.
enum Draw {
    /// One renderer builds the elements and draws them.
    Own(Shared),
    /// The render GPU draws a picture the scanout GPU composes.
    Copy(Box<Bridge>),
}

/// One output's scanout surface.
pub struct RealScanout {
    connector: ConnectorId,
    crtc: crtc::Handle,
    compositor: Compositor,
    draw: Draw,
    flips: Rc<RefCell<Flips>>,
    /// The vblank the next queued frame is for (counted here: the kernel's own sequence is
    /// not used, so a missed vblank is not measured on real hardware; FINDINGS "p1-t").
    next: VblankSeq,
    /// The vblank of the frame whose flip is in flight.
    pending: Option<VblankSeq>,
    /// Held so the device knows this connector's surface is out; dropped with the surface.
    _claim: Rc<()>,
}

/// What a device lends each surface it opens: its buffers, its renderers, its cursor plane size
/// and its flip counts.
pub(super) struct DeviceShare<'a> {
    /// The device's buffer allocator.
    pub gbm: &'a GbmDevice<DrmDeviceFd>,
    /// The renderers that draw it.
    pub feed: Feed,
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
            feed,
            cursor,
            flips,
        } = share;
        let crtc = surface.crtc();
        let allocator = GbmAllocator::new(
            gbm.clone(),
            GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
        );
        let exporter = GbmFramebufferExporter::new(gbm.clone(), NodeFilter::All);
        let size = Size::from((
            i32::try_from(config.mode.size.w).unwrap_or(i32::MAX),
            i32::try_from(config.mode.size.h).unwrap_or(i32::MAX),
        ));
        let scale = f64::from(config.scale.0) / 120.0;
        let transform = transform_of(config.transform);
        let (draw, picture) = match feed {
            Feed::Own(renderer) => (
                Draw::Own(renderer),
                OutputModeSource::Static {
                    size,
                    scale: smithay::utils::Scale::from(scale),
                    transform,
                },
            ),
            Feed::Copy { render, local } => {
                match Bridge::open((render, Rc::clone(&local)), gbm, (size, scale, transform)) {
                    // The picture is already scaled and turned: the compositor shows it as is.
                    Ok(bridge) => (
                        Draw::Copy(Box::new(bridge)),
                        OutputModeSource::Static {
                            size,
                            scale: smithay::utils::Scale::from(1.0),
                            transform: smithay::utils::Transform::Normal,
                        },
                    ),
                    Err(error) => {
                        tracing::warn!(%error, "the render GPU's frames cannot be copied to this output's GPU; this output is drawn by its own GPU");
                        (
                            Draw::Own(local),
                            OutputModeSource::Static {
                                size,
                                scale: smithay::utils::Scale::from(scale),
                                transform,
                            },
                        )
                    }
                }
            }
        };
        let render_formats: Vec<_> = draw
            .scanout_renderer()
            .borrow()
            .egl_context()
            .dmabuf_render_formats()
            .iter()
            .copied()
            .collect();
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
            draw,
            flips,
            next: VblankSeq(0),
            pending: None,
            _claim: claim,
        })
    }

    /// A copy route draws the whole picture next time (the glass lost it, or never got it).
    fn forget_picture(&mut self) {
        if let Draw::Copy(bridge) = &mut self.draw {
            bridge.forget();
        }
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
        let rendered = match &mut self.draw {
            Draw::Own(renderer) => {
                let mut renderer = renderer.borrow_mut();
                let elements = source.gles(&mut renderer);
                self.compositor
                    .render_frame(&mut *renderer, &elements, CLEAR, FrameFlags::DEFAULT)
                    .map(|result| result.is_empty)
                    .map_err(refused)
            }
            Draw::Copy(bridge) => bridge
                .draw(&mut self.compositor, source)
                .map(|drawn| drawn == Drawn::Unchanged),
        };
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
            Err(error) => {
                // What the picture held never reached the glass: draw it all again.
                self.forget_picture();
                QueueOutcome::Failed(error)
            }
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
        self.forget_picture();
        self.compositor.clear().map_err(refused)
    }

    fn reset(&mut self) {
        self.pending = None;
        self.forget_picture();
        if let Err(error) = self.compositor.reset_state() {
            tracing::warn!(%error, "the compositor did not reset after the seat came back");
        }
    }
}

impl Draw {
    /// The renderer of the GPU that scans out: the one the compositor draws with.
    fn scanout_renderer(&self) -> &Shared {
        match self {
            Draw::Own(renderer) => renderer,
            Draw::Copy(bridge) => bridge.local(),
        }
    }
}

/// A compositor failure as the seam's error (the typed sources stay in the log).
pub(super) fn refused(error: impl std::fmt::Display) -> KmsError {
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
