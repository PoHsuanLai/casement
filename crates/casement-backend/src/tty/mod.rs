//! The tty backend (P1-Z declares it; P1-D1 to D3 fill it; P1-T joins the lanes): casement on a
//! real seat, over injected seams.
//!
//! * [`TtySession`], [`DeviceSource`], [`KmsOpener`], [`Kms`] and [`ScanoutSurface`] are the
//!   seams. Each has a thin real adapter (`*_real.rs`, compiled only under feature `tty`) and a
//!   fake in `casement-testkit` (`FakeSession`, `FakeUdev`, `FakeKms`, `FakeScanout`), so a
//!   [`TtyBackend`] runs in the jail with no device at all.
//! * [`TtyDevices`] holds the open display devices; `hotplug::reconcile` turns what a device
//!   reports into the same [`ConnectorEvent`]s the headless backend gives.
//! * [`TtyBackend::apply`] lights an output (a commit, then the device's scanout surface joins the
//!   [`ScanoutSet`]); [`TtyBackend::present`] queues a frame; [`TtyBackend::dispatch`] hears the
//!   seat and hotplug; [`TtyBackend::poll_input`] reads the seat's input devices;
//!   [`TtyBackend::dmabuf_feedback`] is what clients are offered; a suspend arrives through a
//!   [`SleepLink`].
//! * The pure scheduler maths and GPU plan are in `casement_core::{frame, gpu}`.
//!
//! The adapters are the only files that name the device crates (row B7).

mod capture;
mod capture_gpu;
mod connector;
mod device;
mod direct;
mod firmware;
mod flip;
mod hotplug;
mod judge;
mod kms;
mod lit;
mod offer;
mod readback;
mod report;
mod scanouts;
mod session;
mod sleep;
mod surface;
mod udev;
mod vt;
mod writeback;

#[cfg(feature = "tty")]
mod capture_gpu_real;
#[cfg(feature = "tty")]
mod capture_scene_real;
#[cfg(feature = "tty")]
mod direct_real;
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
#[cfg(feature = "tty")]
mod writeback_real;

pub use capture_gpu::{
    BodyCopy, CaptureGpu, CaptureRoute, Picture, SceneSupport, body_copy, capture_fallback,
    capture_route, merged_formats, renderable_formats,
};
pub use connector::{EdidError, VrrPolicy, output_facts, parse_edid, plan_outputs, vrr_state};
pub use device::{Announced, OpenDevice, TtyDevices};
pub use direct::{
    DirectSession, DriverRefusal, FixedNode, NoGpus, POLL_EVERY, VKMS_DRIVER, require_vkms,
};
pub use firmware::{DeviceKinds, SysDeviceKinds};
pub use flip::FlipTracker;
pub use hotplug::{KnownConnectors, reconcile};
pub use judge::{Lit, judge};
pub use kms::{
    CommitVerdict, ConnectorFacts, ConnectorId, ConnectorStatus, CrtcFacts, CrtcId,
    DEFAULT_GAMMA_SIZE, Kms, KmsCommit, KmsError, KmsOpener, KmsRefusal, KmsResources,
    ModesetPolicy, OutputCommit, PlaneFacts, PlaneId, PlaneKind, VrrState,
};
pub use lit::{Lighting, LitOutput};
pub use offer::scanout_formats;
pub use readback::{Readback, Residence, Watch};
pub use report::{Due, KmsReport, REPORT_EVERY, ReportPacer, ReportedOutput};
pub use scanouts::{Flow, FrameTally, PresentOutcome, Scanout, ScanoutSet};
pub use session::{SeatState, SessionNote, TtySession};
pub use sleep::{SleepInbox, SleepLink, SleepSettled, sleep_channel};
pub use surface::{DirectCandidate, Presented, QueueOutcome, QueuedFrame, ScanoutSurface};
pub use udev::{DeviceEvent, DevicePath, DeviceSource};
pub use vt::{VtHeld, VtNumber, vt_of_chord, vt_of_keysym};
pub use writeback::{FLIPS_TO_WAIT, RawCapture, Writeback, WritebackError, WritebackMode};

#[cfg(feature = "tty")]
pub use direct_real::direct_seams;
#[cfg(feature = "tty")]
pub use kms_real::{FdSource, RealKms, RealKmsOpener, VirtualDisplays};
#[cfg(feature = "tty")]
pub use real::real_seams;
#[cfg(feature = "tty")]
pub use session_real::RealSession;
#[cfg(feature = "tty")]
pub use surface_real::RealScanout;
#[cfg(feature = "tty")]
pub use udev_real::RealUdev;

use crate::backend::BackendError;
use crate::gpu::{FormatSource, RenderNodes};
use crate::headless::{ConnectorEvent, FrameSource, PresentedFrame};
use crate::input::{Applied, DeviceSettings, FedInput, InputFeed, PollContext, PresentDevices};
use casement_core::gpu::{
    DisplayKind, GpuChoice, OutputRoute, RenderPlan, keep_displays, plan_for,
};
use casement_core::input::Keysym;
use casement_core::output::{OutputConfig, OutputPower};
use casement_core::settings::PointerSettings;
use casement_core::time::Mono;
use casement_core::{DeviceId, OutputName};
use std::collections::BTreeMap;

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
    /// The formats each render node draws into, for the dma-buf offer (none: the caller offers
    /// no real feedback).
    pub formats: Option<Box<dyn FormatSource>>,
    /// The seat's input devices (none: a seat with no input, as a box with only a remote
    /// session).
    pub input: Option<Box<dyn InputFeed>>,
    /// The device's writeback connector (none: a real seat, which never reads the screen back
    /// this way; `--drm-node` mode on vkms has one).
    pub writeback: Option<Box<dyn Writeback>>,
    /// What each display device is, so a firmware framebuffer is left out beside a real GPU
    /// (none: every device is driven).
    pub kinds: Option<Box<dyn DeviceKinds>>,
}

impl TtySeams {
    /// The seams, with no format source and no input.
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
            formats: None,
            input: None,
            writeback: None,
            kinds: None,
        }
    }

    /// The same with this format source for the dma-buf offer.
    #[must_use]
    pub fn with_formats(self, formats: Box<dyn FormatSource>) -> TtySeams {
        TtySeams {
            formats: Some(formats),
            ..self
        }
    }

    /// The same with this writeback connector.
    #[must_use]
    pub fn with_writeback(self, writeback: Box<dyn Writeback>) -> TtySeams {
        TtySeams {
            writeback: Some(writeback),
            ..self
        }
    }

    /// The same with this reader of device kinds.
    #[must_use]
    pub fn with_kinds(self, kinds: Box<dyn DeviceKinds>) -> TtySeams {
        TtySeams {
            kinds: Some(kinds),
            ..self
        }
    }

    /// The same with these input devices.
    #[must_use]
    pub fn with_input(self, input: Box<dyn InputFeed>) -> TtySeams {
        TtySeams {
            input: Some(input),
            ..self
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
    /// The render plan, when the GPUs were found.
    pub plan: Option<RenderPlan>,
    /// Whether this session holds the seat; `present` draws nothing while it is lent.
    pub seat: SeatState,
    /// The scanout of each lit output.
    pub scanouts: ScanoutSet,
    /// Where each lit output is lit (device, connector, CRTC, mode).
    pub lit: BTreeMap<OutputName, LitOutput>,
    /// The capture shadows, drawn only while a capture is open on the output.
    pub readback: Readback,
    /// Whether to ask for variable refresh where monitor and CRTC can do it.
    pub vrr: VrrPolicy,
    /// How many frames were queued over the run, whatever became of the outputs.
    queued: u64,
    /// The writeback job, when one is queued.
    writeback_job: writeback::WritebackJob,
    /// What each device present or added is (all `Gpu` without a reader of kinds).
    kinds: BTreeMap<DevicePath, DisplayKind>,
    /// The input devices the seat has, and what each was configured with.
    present: PresentDevices,
}

impl TtyBackend {
    /// Takes the injected seams and holds the seat: detects the GPUs and makes the render plan
    /// (none when no GPU was found: the caller offers no real dma-buf feedback), and opens the
    /// display devices udev reports present. Their connectors are announced by
    /// [`announce`](Self::announce), once the loop is up. A device that cannot be opened is
    /// logged and left out.
    ///
    /// ```no_run
    /// use casement_backend::tty::{TtyBackend, TtySeams};
    ///
    /// fn up(seams: TtySeams) {
    ///     let backend = TtyBackend::start(seams).expect("start");
    ///     let _ = (&backend.plan, &backend.devices);
    /// }
    /// ```
    pub fn start(mut seams: TtySeams) -> Result<TtyBackend, BackendError> {
        let plan = match plan_for(&seams.nodes.nodes(), &seams.choice) {
            Ok(plan) => Some(plan),
            Err(refusal) => {
                tracing::warn!(%refusal, "no render plan; dma-buf feedback is not offered");
                None
            }
        };
        let mut devices = TtyDevices::empty();
        let mut kinds = BTreeMap::new();
        let found: Vec<(DevicePath, DisplayKind)> = seams
            .udev
            .present()
            .into_iter()
            .map(|path| {
                let kind = kind_of(seams.kinds.as_deref(), &path);
                kinds.insert(path.clone(), kind);
                (path, kind)
            })
            .collect();
        for path in keep_displays(&found) {
            if let Err(error) = devices.added(path.clone(), &mut *seams.opener) {
                tracing::warn!(path = %path.as_path().display(), %error, "a display device did not open");
            }
        }
        Ok(TtyBackend {
            seams,
            devices,
            plan,
            seat: SeatState::Held,
            scanouts: ScanoutSet::empty(),
            lit: BTreeMap::new(),
            readback: Readback::new(),
            vrr: VrrPolicy::default(),
            queued: 0,
            writeback_job: writeback::WritebackJob::default(),
            kinds,
            present: PresentDevices::default(),
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
                SessionNote::Activated => {
                    returned = true;
                    self.resumed();
                }
                SessionNote::Paused => {
                    returned = false;
                    self.paused();
                }
                SessionNote::VtSwitch(vt) => self.ask_for_vt(VtNumber(vt)),
            }
            self.seams.session.settled(note);
            notes.push(TtyNote::Session(note));
        }
        if self.seat == SeatState::Held {
            if returned {
                notes.extend(self.reread_devices());
            }
            notes.extend(self.read_device_events());
            self.prune();
        }
        notes
    }

    /// The seat was lent away: no frame is queued, the devices give up master, input stops.
    fn paused(&mut self) {
        self.scanouts.flow(Flow::Held);
        self.devices
            .devices
            .values_mut()
            .for_each(|device| device.kms.pause());
        if let Some(input) = &mut self.seams.input {
            input.suspend();
        }
    }

    /// The seat is back: the devices take master, every surface forgets the screen, frames
    /// flow, input reads again. The loop commits every output again (`Effect::ApplyOutput`).
    fn resumed(&mut self) {
        for (path, device) in &mut self.devices.devices {
            if let Err(error) = device.kms.resume() {
                tracing::warn!(path = %path.as_path().display(), %error, "a device did not resume");
            }
        }
        self.scanouts.reset();
        self.scanouts.flow(Flow::Running);
        if let Some(input) = &mut self.seams.input {
            input.resume();
        }
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

    /// Asks the session for VT `vt`.
    pub fn switch_to(&mut self, vt: VtNumber) -> Result<(), BackendError> {
        self.seams.session.switch_vt(vt)?;
        Ok(())
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
                    if !self.admits(&path) {
                        tracing::info!(path = %path.as_path().display(), "a firmware framebuffer is left out beside a GPU");
                        continue;
                    }
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

    /// Whether an added device is driven: not a firmware framebuffer while any device is open.
    fn admits(&mut self, path: &DevicePath) -> bool {
        let kind = kind_of(self.seams.kinds.as_deref(), path);
        self.kinds.insert(path.clone(), kind);
        let found: Vec<(DevicePath, DisplayKind)> = self
            .devices
            .devices
            .keys()
            .map(|open| {
                (
                    open.clone(),
                    self.kinds.get(open).copied().unwrap_or(DisplayKind::Gpu),
                )
            })
            .chain(std::iter::once((path.clone(), kind)))
            .collect();
        keep_displays(&found).contains(path)
    }

    /// Draws `source` for `output` and queues it for the next vblank (`now` is the session
    /// clock's instant); the frame queued (waiting while the seat is paused, the output is
    /// dark or a flip is pending; unchanged when it would show nothing new). While a capture is open on the output the frame is
    /// also drawn into its shadow.
    pub fn present(
        &mut self,
        output: &OutputName,
        source: &dyn FrameSource,
        now: Mono,
    ) -> Result<PresentOutcome, BackendError> {
        let dark = self
            .lit
            .get(output)
            .is_some_and(|lit| lit.power() == OutputPower::Off);
        if self.seat == SeatState::Lent || dark {
            return Ok(PresentOutcome::Waiting);
        }
        let shown = self.scanouts.present(output, source, now)?;
        match shown {
            PresentOutcome::Flipped(seq) => {
                self.queued = self.queued.saturating_add(1);
                self.capture_draw(output, seq, now, source)?;
            }
            // A still desktop flips nothing, yet a capture opened on it needs a first frame.
            PresentOutcome::Unchanged => self.capture_first(output, now, source)?,
            _ => {}
        }
        Ok(shown)
    }

    /// The route `output`'s frames take between GPUs; none without a plan or for a connector it
    /// does not place. The frame loop draws an output with the renderer of
    /// `route.renders_on()` and copies to `route.scans_out_on()` when they differ; today every
    /// output is drawn by its scanout GPU (FINDINGS "wave-2026-10-15/session").
    pub fn route_of(&self, output: &OutputName) -> Option<OutputRoute> {
        self.plan.as_ref()?.route_of(output)
    }

    /// How many frames were queued over the run, on every output.
    pub fn frames_queued(&self) -> u64 {
        self.queued
    }

    /// Starts or stops keeping a read-back of what `output` shows, which has `config` (a
    /// capture session was opened or closed on it).
    pub fn watch(
        &mut self,
        output: &OutputName,
        config: &OutputConfig,
        watch: Watch,
    ) -> Result<(), BackendError> {
        if watch == Watch::Off && self.readback.watching(output) == Watch::On {
            self.capture_stopped(output);
        }
        self.readback.watch(output, config, watch)
    }

    /// The frame `output` last showed, read back; none unless the output is watched and has
    /// shown a frame since.
    pub fn last_frame(&self, output: &OutputName) -> Option<&PresentedFrame> {
        self.readback.last_frame(output)
    }

    /// The dma-buf formats a capture can be drawn into: the display GPU's (with its modifiers)
    /// and, after them, the software renderer's linear ones.
    pub fn capture_formats(&self) -> Vec<smithay::backend::allocator::Format> {
        let gpu = self
            .devices
            .capture_gpu()
            .map(|gpu| gpu.formats())
            .unwrap_or_default();
        merged_formats(gpu, self.readback.capture_formats())
    }

    /// Draws `frame` into a client's dma-buf `target`: with the display GPU's renderer when it
    /// lists the buffer's format and modifier, else (or when binding fails, one log line) with
    /// the software renderer, which binds linear buffers only.
    pub fn write_dmabuf(
        &mut self,
        target: &mut smithay::backend::allocator::dmabuf::Dmabuf,
        frame: &casement_render::pass::CapturedFrame,
    ) -> Result<(), BackendError> {
        use smithay::backend::allocator::Buffer as _;
        let wanted = target.format();
        let mut gpu = self.devices.capture_gpu();
        let listed = gpu.as_ref().map(|gpu| gpu.formats()).unwrap_or_default();
        if let (CaptureRoute::Gpu, Some(gpu)) = (capture_route(wanted, &listed), gpu.as_mut()) {
            match gpu.write_dmabuf(target, frame) {
                Ok(()) => return Ok(()),
                Err(error) => {
                    tracing::info!(%error, "the display GPU did not draw a capture; using the software renderer");
                    if capture_fallback(wanted, &self.readback.capture_formats()).is_none() {
                        return Err(error);
                    }
                }
            }
        }
        self.readback.write_dmabuf(target, frame)
    }

    /// Reads a client's dma-buf `source` back through the software renderer.
    pub fn read_dmabuf(
        &mut self,
        source: &smithay::backend::allocator::dmabuf::Dmabuf,
    ) -> Result<casement_render::pass::ArgbImage, BackendError> {
        self.readback.read_dmabuf(source)
    }

    /// Draws `source` alone into a fresh image of `size` at `scale` through the software
    /// renderer and reads it back: a window's pixels for window capture. A client buffer pixman
    /// cannot import (a dma-buf of a GPU) is missing from it, as from the capture shadow.
    pub fn render_offscreen(
        &mut self,
        size: casement_core::geometry::PhysicalSize,
        scale: ds_core::geometry::scale::Scale,
        source: &dyn FrameSource,
    ) -> Result<crate::headless::RgbaFrame, BackendError> {
        self.readback.render_offscreen(size, scale, source)
    }

    /// What the seat's input devices reported since the last call: nothing while the seat is
    /// lent, or when the seat has no input.
    pub fn poll_input(&mut self, context: &PollContext) -> Vec<FedInput> {
        let fed = match (self.seat, &mut self.seams.input) {
            (SeatState::Held, Some(input)) => input.poll(context),
            _ => Vec::new(),
        };
        for item in &fed {
            match item {
                FedInput::Appeared(device) => self.present.appeared(device.id, device.class),
                FedInput::Gone(id) => self.present.gone(*id),
                _ => {}
            }
        }
        fed
    }

    /// Configures the input device `device` (it appeared, or the settings changed).
    pub fn configure_device(&mut self, device: DeviceId, settings: &DeviceSettings) -> Applied {
        let applied = match &mut self.seams.input {
            Some(input) => input.apply(device, settings),
            None => Applied::Refused,
        };
        if applied != Applied::Refused {
            self.present.applied(device, *settings);
        }
        applied
    }

    /// Configures every pointing device already present that `pointer` no longer matches, so a
    /// changed setting takes effect at once and not after a replug. What each device answered.
    pub fn reconfigure_pointers(&mut self, pointer: &PointerSettings) -> Vec<(DeviceId, Applied)> {
        self.present
            .changes(pointer)
            .into_iter()
            .map(|(device, settings)| (device, self.configure_device(device, &settings)))
            .collect()
    }
}

/// What a device reported, as notes; a device that refused is logged and reports nothing.
/// The kind of the device at `path` under `kinds`; a GPU when nothing says otherwise.
fn kind_of(kinds: Option<&dyn DeviceKinds>, path: &DevicePath) -> DisplayKind {
    kinds.map_or(DisplayKind::Gpu, |kinds| kinds.kind(path))
}

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
