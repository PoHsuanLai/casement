//! The tty backend (P1-Z declares it; P1-D1 to D3 fill it): casement on a real seat, over
//! injected seams.
//!
//! * [`TtySession`], [`DeviceSource`], [`KmsOpener`], [`Kms`] and [`ScanoutSurface`] are the
//!   seams. Each has a thin real adapter (`*_real.rs`, compiled only under feature `tty`) and a
//!   fake in `casement-testkit` (`FakeSession`, `FakeUdev`, `FakeKms`, `FakeScanout`), so a
//!   [`TtyBackend`] runs in the jail with no device at all.
//! * [`TtyDevices`] holds the open display devices; `hotplug::reconcile` turns what a device
//!   reports into the same [`ConnectorEvent`]s the headless backend gives.
//! * The pure scheduler maths and GPU plan are in `casement_core::{frame, gpu}`.
//!
//! The adapters are the only files that name the device crates (row B7).

mod connector;
mod device;
mod flip;
mod hotplug;
mod kms;
mod scanouts;
mod session;
mod surface;
mod udev;
mod vt;

#[cfg(feature = "tty")]
mod kms_real;
#[cfg(feature = "tty")]
mod real;
#[cfg(feature = "tty")]
mod session_real;
#[cfg(feature = "tty")]
mod surface_real;
#[cfg(feature = "tty")]
mod udev_real;

pub use connector::{EdidError, output_facts, parse_edid};
pub use device::{OpenDevice, TtyDevices};
pub use flip::FlipTracker;
pub use hotplug::{KnownConnectors, reconcile};
pub use kms::{
    CommitVerdict, ConnectorFacts, ConnectorId, ConnectorStatus, CrtcFacts, CrtcId, Kms, KmsCommit,
    KmsError, KmsOpener, KmsRefusal, KmsResources, ModesetPolicy, OutputCommit, PlaneFacts,
    PlaneId, PlaneKind, VrrState,
};
pub use scanouts::{Flow, FrameTally, Scanout, ScanoutSet};
pub use session::{SessionNote, TtySession};
pub use surface::{DirectCandidate, Presented, QueueOutcome, QueuedFrame, ScanoutSurface};
pub use udev::{DeviceEvent, DevicePath, DeviceSource};
pub use vt::{VtNumber, vt_of_keysym};

#[cfg(feature = "tty")]
pub use kms_real::{RealKms, RealKmsOpener};
#[cfg(feature = "tty")]
pub use real::real_seams;
#[cfg(feature = "tty")]
pub use session_real::RealSession;
#[cfg(feature = "tty")]
pub use surface_real::RealScanout;
#[cfg(feature = "tty")]
pub use udev_real::RealUdev;

use crate::backend::BackendError;
use crate::gpu::RenderNodes;
use crate::headless::{ConnectorEvent, FrameSource};
use casement_core::OutputName;
use casement_core::gpu::{GpuChoice, RenderPlan};
use casement_core::time::VblankSeq;

/// Everything the tty backend is injected with.
#[derive(Debug)]
#[non_exhaustive]
pub struct TtySeams {
    /// Who opens devices and hears the seat change hands.
    pub session: Box<dyn TtySession>,
    /// Which display devices exist and change.
    pub udev: Box<dyn DeviceSource>,
    /// Opens a display device.
    pub opener: Box<dyn KmsOpener>,
    /// The GPUs, for the render plan.
    pub nodes: Box<dyn RenderNodes>,
    /// The `gpu.render` setting.
    pub choice: GpuChoice,
}

impl TtySeams {
    /// The seams.
    pub fn new(
        session: Box<dyn TtySession>,
        udev: Box<dyn DeviceSource>,
        opener: Box<dyn KmsOpener>,
        nodes: Box<dyn RenderNodes>,
        choice: GpuChoice,
    ) -> TtySeams {
        TtySeams {
            session,
            udev,
            opener,
            nodes,
            choice,
        }
    }
}

/// What the loop learns when it drains the tty backend.
#[derive(Debug)]
#[non_exhaustive]
pub enum TtyNote {
    /// A connector appeared, changed or went.
    Connector(ConnectorEvent),
    /// The seat changed hands.
    Session(SessionNote),
}

/// The running tty backend.
#[derive(Debug)]
pub struct TtyBackend {
    /// What it was injected with.
    pub seams: TtySeams,
    /// The display devices it holds.
    pub devices: TtyDevices,
    /// The render plan, once the GPUs are known.
    pub plan: Option<RenderPlan>,
    /// The scanout of each lit output (`ScanoutSet::default()` at start; P1-D2 attaches).
    pub scanouts: ScanoutSet,
}

impl TtyBackend {
    /// Opens every display device present, makes the render plan and holds the seat.
    pub fn start(seams: TtySeams) -> Result<TtyBackend, BackendError> {
        let _ = seams;
        todo!("P1-D1: TtyBackend::start")
    }

    /// The connector events of the devices present at start, once (then none).
    pub fn announce(&mut self) -> Vec<ConnectorEvent> {
        todo!("P1-D2: TtyBackend::announce")
    }

    /// Reads the seat and device events that arrived: a VT switch pauses, hotplug reconciles.
    pub fn dispatch(&mut self) -> Vec<TtyNote> {
        todo!("P1-D1: TtyBackend::dispatch")
    }

    /// Draws `source` for `output` and queues it for the next vblank; the number of the frame
    /// queued (none while the seat is paused or a flip is pending).
    pub fn present(
        &mut self,
        output: &OutputName,
        source: &dyn FrameSource,
    ) -> Result<Option<VblankSeq>, BackendError> {
        self.scanouts.present(output, source)
    }
}
