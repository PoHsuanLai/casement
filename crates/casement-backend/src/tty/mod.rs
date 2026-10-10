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
mod judge;
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

pub use connector::{EdidError, VrrPolicy, output_facts, parse_edid, plan_outputs, vrr_state};
pub use device::{Announced, OpenDevice, TtyDevices};
pub use flip::FlipTracker;
pub use hotplug::{KnownConnectors, reconcile};
pub use judge::{Lit, judge};
pub use kms::{
    CommitVerdict, ConnectorFacts, ConnectorId, ConnectorStatus, CrtcFacts, CrtcId, Kms, KmsCommit,
    KmsError, KmsOpener, KmsRefusal, KmsResources, ModesetPolicy, OutputCommit, PlaneFacts,
    PlaneId, PlaneKind, VrrState,
};
pub use scanouts::{Flow, FrameTally, Scanout, ScanoutSet};
pub use session::{SeatState, SessionNote, TtySession};
pub use surface::{DirectCandidate, Presented, QueueOutcome, QueuedFrame, ScanoutSurface};
pub use udev::{DeviceEvent, DevicePath, DeviceSource};
pub use vt::{VtNumber, vt_of_keysym};

#[cfg(feature = "tty")]
pub use kms_real::{FdSource, RealKms, RealKmsOpener};
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
use casement_core::input::Keysym;
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
    /// Whether this session holds the seat; `present` draws nothing while it is lent.
    pub seat: SeatState,
    /// The scanout of each lit output (`ScanoutSet::default()` at start; P1-D2 attaches).
    pub scanouts: ScanoutSet,
}

impl TtyBackend {
    /// Takes the injected seams and holds the seat. The devices present are opened by
    /// [`announce`](Self::announce), and the render plan is made from the GPUs once they are known
    /// (P1-D4), so a start touches no device.
    ///
    /// ```no_run
    /// use casement_backend::tty::{TtyBackend, TtySeams};
    ///
    /// fn up(seams: TtySeams) {
    ///     let backend = TtyBackend::start(seams).expect("start");
    ///     assert!(backend.devices.devices.is_empty());
    /// }
    /// ```
    pub fn start(seams: TtySeams) -> Result<TtyBackend, BackendError> {
        Ok(TtyBackend {
            seams,
            devices: TtyDevices::empty(),
            plan: None,
            seat: SeatState::Held,
            scanouts: ScanoutSet::empty(),
        })
    }

    /// The connector events of the devices present at start, once (then none).
    pub fn announce(&mut self) -> Vec<ConnectorEvent> {
        self.devices.announce()
    }

    /// Reads the seat and device events that arrived, in order: a VT switch pauses, a return
    /// re-reads every open device, hotplug reconciles. While the seat is lent no device is
    /// touched: the device events stay queued and are read after the return.
    pub fn dispatch(&mut self) -> Vec<TtyNote> {
        let mut notes = Vec::new();
        let mut returned = false;
        for note in self.seams.session.notes() {
            self.seat = self.seat.after(note);
            match note {
                SessionNote::Activated => returned = true,
                SessionNote::Paused => returned = false,
                SessionNote::VtSwitch(vt) => self.ask_for_vt(VtNumber(vt)),
                _ => {}
            }
            notes.push(TtyNote::Session(note));
        }
        if self.seat == SeatState::Held {
            if returned {
                notes.extend(self.reread_devices());
            }
            notes.extend(self.read_device_events());
        }
        notes
    }

    /// Asks the session for the VT `sym` names (a `Ctrl+Alt+Fn` chord, after xkb); the VT asked
    /// for, or none when `sym` names no VT. The session's own pause note follows when the
    /// switch happens.
    pub fn request_vt(&mut self, sym: Keysym) -> Result<Option<VtNumber>, BackendError> {
        let Some(vt) = vt_of_keysym(sym) else {
            return Ok(None);
        };
        self.seams.session.switch_vt(vt)?;
        Ok(Some(vt))
    }

    /// A switch the session itself reported as wanted; a refusal is logged, not fatal.
    fn ask_for_vt(&mut self, vt: VtNumber) {
        if let Err(fault) = self.seams.session.switch_vt(vt) {
            tracing::warn!(vt = vt.0, %fault, "the VT switch was refused");
        }
    }

    /// Every open device's connectors, read again (the seat came back; a display may have
    /// changed while it was away).
    fn reread_devices(&mut self) -> Vec<TtyNote> {
        let paths: Vec<DevicePath> = self.devices.devices.keys().cloned().collect();
        paths
            .into_iter()
            .flat_map(|path| {
                let result = self.devices.changed(&path);
                connector_notes(&path, result)
            })
            .collect()
    }

    /// The device events waiting, applied in order.
    fn read_device_events(&mut self) -> Vec<TtyNote> {
        let mut notes = Vec::new();
        while let Some(event) = self.seams.udev.next_event() {
            let (path, result) = match event {
                DeviceEvent::Added(path) => {
                    let result = self.devices.added(path.clone(), &mut *self.seams.opener);
                    (path, result)
                }
                DeviceEvent::Changed(path) => {
                    let result = self.devices.changed(&path);
                    (path, result)
                }
                DeviceEvent::Removed(path) => {
                    let events = self.devices.removed(&path);
                    (path, Ok(events))
                }
            };
            notes.extend(connector_notes(&path, result));
        }
        notes
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

/// What a device reported, as notes; a device that refused is logged and reports nothing.
fn connector_notes(
    path: &DevicePath,
    result: Result<Vec<ConnectorEvent>, KmsError>,
) -> Vec<TtyNote> {
    match result {
        Ok(events) => events.into_iter().map(TtyNote::Connector).collect(),
        Err(error) => {
            tracing::warn!(path = %path.as_path().display(), %error, "a device refused");
            Vec::new()
        }
    }
}
