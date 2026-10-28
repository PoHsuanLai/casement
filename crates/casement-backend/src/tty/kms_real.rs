//! The real KMS device (feature `tty`): `Kms` over smithay's DRM device, a thin adapter (B7).
//! Nothing else in the workspace names the device crates.
//!
//! The device is opened without disabling its connectors, so a handover from the boot console
//! does not flicker. A commit lights outputs by creating a `DrmSurface` per connector, which
//! [`Kms::scanout`] hands to the scanout adapter (`surface_real.rs`) to wrap in a
//! `DrmCompositor`, the thing that owns buffers and the kernel's own atomic test; the device's
//! vblank source is inserted into the event loop at open and fills the [`Flips`] the surfaces
//! read. This file checks a commit by the pure rules in [`judge`](super::judge) and does the
//! modeset bookkeeping; it cannot `TEST_ONLY` a commit that has no framebuffer yet. A commit
//! for a connector whose surface is out (re-applied after a VT switch) changes nothing: the
//! compositor that owns the surface keeps its mode, and a new mode is a new surface (the
//! backend drops the old one first).
//!
//! An output is drawn by the GPU (GLES, `surface_real.rs`) or by the CPU (pixman over dumb
//! buffers, `soft_real.rs`); [`Routes`] chooses (`render_mode.rs`, CASB-07) and
//! `kms_real/makers.rs` makes the surface. A device opened for software drawing opens no GBM
//! device at all.

use super::capture_gpu::CaptureGpu;
use super::capture_gpu_real::RealCaptureGpu;
use super::capture_scene_real::SceneStore;
use super::direct::require_vkms;
use super::hotplug::{KnownConnectors, reconcile};
use super::judge::{Lit, judge};
use super::kms::{
    CommitVerdict, ConnectorFacts, ConnectorId, ConnectorStatus, CrtcFacts, CrtcId, Kms, KmsCommit,
    KmsError, KmsOpener, KmsRefusal, KmsResources, OutputCommit, PlaneFacts, PlaneId, PlaneKind,
};
use super::render_hub_real::RenderHub;
use super::render_mode::{RenderMode, Routes};
use super::soft_real::SharedPixman;
use super::surface::ScanoutSurface;
use super::surface_real::{Feed, Flips};
use super::udev::DevicePath;
use crate::headless::ConnectorEvent;
use casement_core::OutputName;
use casement_core::geometry::PhysicalSize;
use casement_core::gpu::{FormatEntry, Fourcc, Modifier, OutputRoute};
use casement_core::output::{
    GammaRamp, Mode, ModePreference, OutputConfig, OutputPower, PhysicalMm, VrrCapability,
};
use casement_core::time::Millihertz;
use smithay::backend::drm::{DrmDevice, DrmDeviceFd, DrmEvent, DrmNode, DrmSurface, NodeType};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::drm::Device as BasicDevice;
use smithay::reexports::drm::control::{
    self, Device as ControlDevice, ModeTypeFlags, PlaneType, ResourceHandle, connector, crtc,
};
use smithay::reexports::gbm::Device as GbmDevice;
use smithay::utils::DeviceFd;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::os::fd::OwnedFd;
use std::rc::{Rc, Weak};

mod makers;

use makers::Makers;

/// Where a device's file descriptor comes from: the seat session (logind or seatd), which alone
/// may open a DRM node without root. `session_real.rs` provides the implementation.
pub trait FdSource: std::fmt::Debug {
    /// The opened node at `path`.
    fn fd(&mut self, path: &DevicePath) -> Result<OwnedFd, KmsError>;
}

/// A display device opened through the session.
#[derive(Debug)]
pub struct RealKms {
    device: DrmDevice,
    /// The device's descriptor, which the software surfaces allocate dumb buffers on.
    fd: DrmDeviceFd,
    /// The GBM device GLES surfaces allocate on; none in software mode, and none when it could
    /// not be opened (every output is then drawn by the CPU).
    gbm: Option<GbmDevice<DrmDeviceFd>>,
    /// Which way each output is drawn.
    routes: Routes,
    /// The CPU's renderer, made for the first software surface and shared by the rest.
    pixman: Option<SharedPixman>,
    flips: Rc<RefCell<Flips>>,
    /// The device's renderer, made for the first scanout and shared by the rest.
    renderer: Option<Rc<RefCell<GlesRenderer>>>,
    /// The frames kept on the GPU for the outputs a capture watches.
    scenes: Rc<RefCell<SceneStore>>,
    /// The render node of the device, when it has one: what clients allocate buffers on.
    node: Option<u64>,
    /// The render GPU's renderer and the plan's routes; none draws every output with the
    /// device's own renderer.
    hub: Option<Rc<RenderHub>>,
    /// The surfaces lit and not yet handed to a scanout.
    surfaces: BTreeMap<ConnectorId, DrmSurface>,
    /// The connectors whose surface is out with a scanout (gone when the scanout is dropped).
    out: BTreeMap<ConnectorId, Weak<()>>,
    lit: Lit,
    power: BTreeMap<ConnectorId, OutputPower>,
    known: KnownConnectors,
    last: KmsResources,
}

/// The number of the render node of the device behind `fd` (its primary node's sibling), or the
/// node itself when it has no render node.
fn render_node_of(fd: &OwnedFd) -> Option<u64> {
    let node = DrmNode::from_file(fd).ok()?;
    let render = node
        .node_with_type(NodeType::Render)
        .and_then(Result::ok)
        .unwrap_or(node);
    Some(render.dev_id())
}

fn io_error(error: impl std::error::Error + Send + Sync + 'static) -> KmsError {
    KmsError::Io(io::Error::other(error))
}

fn io_error_text(text: &str) -> KmsError {
    KmsError::Io(io::Error::other(text.to_owned()))
}

/// The GBM device on `fd`; none (said in the log) when it cannot be opened, and every output of
/// the device is then drawn by the CPU.
fn open_gbm(fd: &DrmDeviceFd) -> Option<GbmDevice<DrmDeviceFd>> {
    GbmDevice::new(fd.clone())
        .inspect_err(|error| {
            tracing::warn!(%error, "no GBM device on the display device; its outputs are drawn by the CPU");
        })
        .ok()
}

impl RealKms {
    /// Wraps the device behind `fd`: reads its resources, takes master, and has `handle`'s loop
    /// read the device's vblank events.
    pub fn open<D: 'static>(
        fd: OwnedFd,
        handle: &LoopHandle<'static, D>,
    ) -> Result<RealKms, KmsError> {
        RealKms::open_routed(fd, handle, None, RenderMode::Gpu)
    }

    /// The same, with the render GPU's hub: the device draws as the plan's route for its GPU
    /// says (a frame drawn on the render GPU and copied across when another GPU renders). In
    /// [`RenderMode::Software`] no GBM device is opened and the hub is never asked for a
    /// renderer.
    pub fn open_routed<D: 'static>(
        fd: OwnedFd,
        handle: &LoopHandle<'static, D>,
        hub: Option<Rc<RenderHub>>,
        mode: RenderMode,
    ) -> Result<RealKms, KmsError> {
        let node = render_node_of(&fd);
        let fd = DrmDeviceFd::new(DeviceFd::from(fd));
        let (device, notifier) = DrmDevice::new(fd.clone(), false).map_err(io_error)?;
        let gbm = match mode {
            RenderMode::Software => None,
            RenderMode::Gpu => open_gbm(&fd),
        };
        let flips = Rc::new(RefCell::new(Flips::default()));
        let into = Rc::clone(&flips);
        handle
            .insert_source(notifier, move |event, _, _| match event {
                DrmEvent::VBlank(crtc) => into.borrow_mut().done(crtc),
                DrmEvent::Error(error) => {
                    tracing::warn!(%error, "the display device reported an error")
                }
            })
            .map_err(|error| io_error(error.error))?;
        let last = probe(&device)?;
        let known = reconcile(&KnownConnectors::default(), &last).0;
        Ok(RealKms {
            device,
            fd,
            gbm,
            routes: Routes::new(mode),
            pixman: None,
            flips,
            renderer: None,
            scenes: Rc::default(),
            node,
            hub,
            surfaces: BTreeMap::new(),
            out: BTreeMap::new(),
            lit: Lit::new(),
            power: BTreeMap::new(),
            known,
            last,
        })
    }

    /// The route of this device's frames under the plan; none without a hub, without a render
    /// node, or for a GPU the plan scans no output out on (drawn with its own renderer).
    fn route(&self) -> Option<OutputRoute> {
        self.hub.as_ref()?.route_of_device(self.node?)
    }

    /// The GBM device GLES surfaces allocate on.
    fn gbm(&self) -> Result<&GbmDevice<DrmDeviceFd>, KmsError> {
        self.gbm
            .as_ref()
            .ok_or_else(|| io_error_text("this device has no GBM device"))
    }

    /// The device's GLES renderer, made on first use. The render GPU's device shares the
    /// hub's renderer (the one every copy route draws with); any other makes its own.
    fn renderer(&mut self) -> Result<Rc<RefCell<GlesRenderer>>, KmsError> {
        if let Some(renderer) = &self.renderer {
            return Ok(Rc::clone(renderer));
        }
        let shared = match (&self.hub, self.route()) {
            (Some(hub), Some(OutputRoute::Direct { .. })) => hub.renderer(Some(self.gbm()?))?,
            _ => {
                let renderer = casement_render::gles_on(self.gbm()?.clone())
                    .map_err(|error| KmsError::Io(io::Error::other(error.to_string())))?;
                Rc::new(RefCell::new(renderer))
            }
        };
        self.renderer = Some(Rc::clone(&shared));
        Ok(shared)
    }

    /// The renderers that draw this device's outputs: the render GPU's and this device's own
    /// when the plan copies, else the device's own. A copy route that cannot start (the render
    /// GPU's renderer did not come up) is logged and the device draws for itself.
    fn feed(&mut self) -> Result<Feed, KmsError> {
        let local = self.renderer()?;
        let (Some(hub), Some(OutputRoute::Copy { render, scanout })) = (&self.hub, self.route())
        else {
            return Ok(Feed::Own(local));
        };
        Ok(match hub.renderer(None) {
            Ok(render) => Feed::Copy { render, local },
            Err(error) => {
                tracing::warn!(%error, ?render, ?scanout, "the render GPU did not start; its outputs are drawn by their own GPU");
                Feed::Own(local)
            }
        })
    }

    /// Whether the surface of `connector` is out with a live scanout.
    fn is_out(&self, connector: ConnectorId) -> bool {
        self.out
            .get(&connector)
            .is_some_and(|claim| claim.strong_count() > 0)
    }

    fn connector_handle(&self, id: ConnectorId) -> Option<connector::Handle> {
        let handles = self.device.resource_handles().ok()?;
        handles
            .connectors()
            .iter()
            .copied()
            .find(|h| u32::from(*h) == id.0)
    }

    fn crtc_handle(&self, id: CrtcId) -> Option<crtc::Handle> {
        self.device
            .crtcs()
            .iter()
            .copied()
            .find(|h| u32::from(*h) == id.0)
    }

    /// The raw mode of `connector` that reads as `mode`.
    fn raw_mode(&self, handle: connector::Handle, mode: &Mode) -> Option<control::Mode> {
        let info = self.device.get_connector(handle, false).ok()?;
        info.modes()
            .iter()
            .find(|raw| {
                let seen = mode_of(raw);
                seen.size == mode.size && seen.refresh == mode.refresh
            })
            .copied()
    }

    /// Lights one output: a new surface, or the surface it already has re-aimed.
    fn light(&mut self, output: &OutputCommit) -> Result<(), KmsError> {
        let connector = self
            .connector_handle(output.connector)
            .ok_or(KmsError::UnknownConnector(output.connector))?;
        let crtc = self.crtc_handle(output.crtc).ok_or(KmsRefusal::NoCrtc)?;
        let raw = self
            .raw_mode(connector, &output.mode)
            .ok_or(KmsRefusal::Mode)?;
        if self.is_out(output.connector) {
            // The compositor that owns the surface keeps its mode; a new mode arrives as a new
            // surface, after the backend dropped the old scanout.
            return Ok(());
        }
        if let Some(surface) = self.surfaces.get(&output.connector) {
            surface.use_mode(raw).map_err(io_error)?;
            surface.set_connectors(&[connector]).map_err(io_error)?;
            surface.use_vrr(output.vrr).map_err(io_error)?;
        } else {
            let surface = self
                .device
                .create_surface(crtc, raw, &[connector])
                .map_err(io_error)?;
            surface.use_vrr(output.vrr).map_err(io_error)?;
            self.surfaces.insert(output.connector, surface);
        }
        Ok(())
    }
}

impl Kms for RealKms {
    fn capture_gpu(&self) -> Option<Box<dyn CaptureGpu>> {
        // Captures draw the scene with the renderer that built it: the render GPU's, when this
        // device's outputs are copied from there.
        let (renderer, node) = match (&self.hub, self.route()) {
            (Some(hub), Some(OutputRoute::Copy { render, .. })) => {
                (hub.existing()?, Some(render.dev_t()))
            }
            _ => (Rc::clone(self.renderer.as_ref()?), self.node),
        };
        Some(Box::new(RealCaptureGpu::new(
            renderer,
            Rc::clone(&self.scenes),
            node,
        )))
    }

    fn resources(&self) -> KmsResources {
        probe(&self.device).unwrap_or_else(|_| self.last.clone())
    }

    fn test(&mut self, commit: &KmsCommit) -> CommitVerdict {
        judge(&self.resources(), &self.lit, commit)
    }

    fn commit(&mut self, commit: KmsCommit) -> Result<(), KmsError> {
        match self.test(&commit) {
            CommitVerdict::Accepted => {}
            CommitVerdict::NeedsModeset => {
                return Err(KmsError::Refused(KmsRefusal::ModesetForbidden));
            }
            CommitVerdict::Refused(refusal) => return Err(KmsError::Refused(refusal)),
        }
        for output in &commit.outputs {
            self.light(output)?;
            self.lit.insert(output.connector, output.clone());
        }
        Ok(())
    }

    fn set_power(&mut self, connector: ConnectorId, power: OutputPower) -> Result<(), KmsError> {
        if !self
            .resources()
            .connectors
            .iter()
            .any(|c| c.id == connector)
        {
            return Err(KmsError::UnknownConnector(connector));
        }
        // Off clears a surface still in hand (CRTC inactive); one out with a scanout is
        // cleared by the scanout (`ScanoutSurface::blank`). The surface comes back on the next
        // frame the scanout queues, so On only records the intent.
        if let (OutputPower::Off, Some(surface)) = (power, self.surfaces.get(&connector)) {
            surface.clear().map_err(io_error)?;
        }
        self.power.insert(connector, power);
        Ok(())
    }

    fn set_gamma(&mut self, connector: ConnectorId, ramp: &GammaRamp) -> Result<(), KmsError> {
        let id = self
            .lit
            .get(&connector)
            .map(|lit| lit.crtc)
            .ok_or(KmsError::UnknownConnector(connector))?;
        let crtc = self.crtc_handle(id).ok_or(KmsRefusal::NoCrtc)?;
        let size = self.device.get_crtc(crtc)?.gamma_length() as usize;
        let [red, green, blue] = channels(ramp, size);
        self.device.set_gamma(crtc, &red, &green, &blue)?;
        Ok(())
    }

    fn events(&mut self) -> Vec<ConnectorEvent> {
        let now = self.resources();
        let (known, events) = reconcile(&self.known, &now);
        self.known = known;
        self.last = now;
        events
    }

    fn scanout(
        &mut self,
        connector: ConnectorId,
        config: &OutputConfig,
    ) -> Result<Box<dyn ScanoutSurface>, KmsError> {
        let mut routes = std::mem::take(&mut self.routes);
        let opened = routes.open(&mut Makers { kms: self }, connector, config);
        self.routes = routes;
        let opened = opened?;
        tracing::debug!(?connector, drawing = ?opened.drawing, "a scanout surface was made");
        Ok(opened.surface)
    }

    fn pause(&mut self) {
        self.device.pause();
    }

    fn resume(&mut self) -> Result<(), KmsError> {
        self.device.activate(false).map_err(io_error)
    }
}

/// The three channels of `ramp` at `size` steps: the identity is a straight line, a table is
/// resampled to the device's size by nearest step.
fn channels(ramp: &GammaRamp, size: usize) -> [Vec<u16>; 3] {
    let line = |step: usize| {
        let top = size.saturating_sub(1).max(1);
        u16::try_from(step * usize::from(u16::MAX) / top).unwrap_or(u16::MAX)
    };
    let sample = |table: &[u16], step: usize| {
        let at = step * table.len() / size.max(1);
        table.get(at).copied().unwrap_or(u16::MAX)
    };
    match ramp {
        GammaRamp::Table(table) => [table.red(), table.green(), table.blue()]
            .map(|channel| (0..size).map(|step| sample(channel, step)).collect()),
        _ => [(); 3].map(|()| (0..size).map(line).collect()),
    }
}

/// A mode as casement reads it: size, refresh in millihertz from the timings (the kernel's
/// own rounded `vrefresh` when they are zero), preferred as the display marks it.
fn mode_of(raw: &control::Mode) -> Mode {
    let (w, h) = raw.size();
    let (_, _, htotal) = raw.hsync();
    let (_, _, vtotal) = raw.vsync();
    let pixels = u64::from(htotal) * u64::from(vtotal);
    let refresh = match pixels {
        0 => raw.vrefresh().saturating_mul(1000),
        pixels => u32::try_from(u64::from(raw.clock()) * 1_000_000 / pixels).unwrap_or(u32::MAX),
    };
    let preference = match raw.mode_type().contains(ModeTypeFlags::PREFERRED) {
        true => ModePreference::Preferred,
        false => ModePreference::Other,
    };
    Mode {
        size: PhysicalSize {
            w: u32::from(w),
            h: u32::from(h),
        },
        refresh: Millihertz(refresh),
        preference,
    }
}

/// The raw value of the property `name` on `handle`, when it has one.
fn property(device: &DrmDevice, handle: impl ResourceHandle, name: &str) -> Option<u64> {
    let set = device.get_properties(handle).ok()?;
    let (ids, values) = set.as_props_and_values();
    ids.iter().zip(values).find_map(|(id, value)| {
        let info = device.get_property(*id).ok()?;
        (info.name().to_bytes() == name.as_bytes()).then_some(*value)
    })
}

fn capability(present: bool) -> VrrCapability {
    match present {
        true => VrrCapability::Capable,
        false => VrrCapability::Incapable,
    }
}

/// Reads every connector, CRTC and plane the device has now.
fn probe(device: &DrmDevice) -> Result<KmsResources, KmsError> {
    let handles = device.resource_handles()?;
    let crtcs: Vec<CrtcFacts> = handles
        .crtcs()
        .iter()
        .map(|h| {
            let vrr = property(device, *h, "VRR_ENABLED").is_some();
            let gamma = device.get_crtc(*h).map_or(0, |info| info.gamma_length());
            CrtcFacts::new(CrtcId(u32::from(*h)), capability(vrr)).with_gamma_size(gamma)
        })
        .collect();
    let connectors = handles
        .connectors()
        .iter()
        .filter_map(|h| connector_facts(device, &handles, *h).transpose())
        .collect::<Result<Vec<_>, _>>()?;
    let planes = planes(device, handles.crtcs())?;
    Ok(KmsResources::new(connectors, crtcs, planes))
}

/// One connector's facts; none for a connector whose name casement cannot spell.
fn connector_facts(
    device: &DrmDevice,
    handles: &control::ResourceHandles,
    handle: connector::Handle,
) -> Result<Option<ConnectorFacts>, KmsError> {
    let info = device.get_connector(handle, false)?;
    // A writeback connector (listed once the writeback seam asked for them) is not a screen.
    if info.interface() == connector::Interface::Writeback {
        return Ok(None);
    }
    let text = format!("{}-{}", info.interface().as_str(), info.interface_id());
    let Ok(name) = OutputName::parse(&text) else {
        return Ok(None);
    };
    let status = match info.state() {
        connector::State::Connected => ConnectorStatus::Connected,
        connector::State::Disconnected => ConnectorStatus::Disconnected,
        _ => ConnectorStatus::Unknown,
    };
    let (w, h) = info.size().unwrap_or((0, 0));
    let edid = property(device, handle, "EDID")
        .filter(|blob| *blob != 0)
        .and_then(|blob| device.get_property_blob(blob).ok())
        .unwrap_or_default();
    let vrr = property(device, handle, "vrr_capable").is_some_and(|value| value != 0);
    let possible = info
        .encoders()
        .iter()
        .filter_map(|encoder| device.get_encoder(*encoder).ok())
        .flat_map(|encoder| handles.filter_crtcs(encoder.possible_crtcs()))
        .map(|h| CrtcId(u32::from(h)))
        .collect::<std::collections::BTreeSet<_>>();
    Ok(Some(
        ConnectorFacts::new(ConnectorId(u32::from(handle)), name, status)
            .with_modes(info.modes().iter().map(mode_of).collect())
            .with_physical(PhysicalMm { w, h })
            .with_edid(edid)
            .with_vrr(capability(vrr))
            .with_crtcs(possible.into_iter().collect()),
    ))
}

/// Every plane once, with the CRTCs it can sit on.
fn planes(device: &DrmDevice, crtcs: &[crtc::Handle]) -> Result<Vec<PlaneFacts>, KmsError> {
    let mut found: BTreeMap<PlaneId, PlaneFacts> = BTreeMap::new();
    for crtc in crtcs {
        let on = CrtcId(u32::from(*crtc));
        let sets = device.planes(crtc).map_err(io_error)?;
        let all = sets.primary.iter().chain(&sets.overlay).chain(&sets.cursor);
        for plane in all {
            let id = PlaneId(u32::from(plane.handle));
            let kind = match plane.type_ {
                PlaneType::Primary => PlaneKind::Primary,
                PlaneType::Cursor => PlaneKind::Cursor,
                _ => PlaneKind::Overlay,
            };
            let formats = plane
                .formats
                .iter()
                .map(|f| FormatEntry {
                    format: Fourcc(f.code as u32),
                    modifier: Modifier(u64::from(f.modifier)),
                })
                .collect();
            found
                .entry(id)
                .or_insert_with(|| PlaneFacts::new(id, kind, Vec::new(), formats))
                .possible_crtcs
                .push(on);
        }
    }
    Ok(found.into_values().collect())
}

/// Opens devices through a session (`session_real.rs` hands the descriptors), each with its
/// vblank source in `handle`'s loop.
pub struct RealKmsOpener<D: 'static> {
    fds: Box<dyn FdSource>,
    handle: LoopHandle<'static, D>,
    /// Whether a virtual test display (a device whose DRM driver is vkms) is refused: a session
    /// drives real displays only; `--drm-node` mode allows it and checks the driver itself.
    refuse_virtual: bool,
    hub: Option<Rc<RenderHub>>,
    mode: RenderMode,
}

impl<D: 'static> RealKmsOpener<D> {
    /// An opener that gets descriptors from `fds` and has `handle`'s loop read each device.
    pub fn new(fds: Box<dyn FdSource>, handle: LoopHandle<'static, D>) -> RealKmsOpener<D> {
        RealKmsOpener {
            fds,
            handle,
            refuse_virtual: false,
            hub: None,
            mode: RenderMode::Gpu,
        }
    }

    /// The same, drawing every output as `mode` says: [`RenderMode::Software`] opens no GBM
    /// device and no GPU renderer on any device (`--safe`).
    #[must_use]
    pub fn drawing(self, mode: RenderMode) -> RealKmsOpener<D> {
        RealKmsOpener { mode, ..self }
    }

    /// The same, drawing as the plan behind `hub` says (the render GPU renders every output).
    #[must_use]
    pub fn routed(self, hub: Rc<RenderHub>) -> RealKmsOpener<D> {
        RealKmsOpener {
            hub: Some(hub),
            ..self
        }
    }

    /// The same, refusing to open a virtual test display (a session's opener).
    #[must_use]
    pub fn refusing_virtual(self) -> RealKmsOpener<D> {
        RealKmsOpener {
            refuse_virtual: true,
            ..self
        }
    }
}

impl<D: 'static> std::fmt::Debug for RealKmsOpener<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealKmsOpener")
            .field("fds", &self.fds)
            .finish_non_exhaustive()
    }
}

impl<D: 'static> KmsOpener for RealKmsOpener<D> {
    fn open(&mut self, path: &DevicePath) -> Result<Box<dyn Kms>, KmsError> {
        let fd = self.fds.fd(path)?;
        if self.refuse_virtual && is_vkms(&fd)? {
            return Err(KmsError::VirtualDisplay(path.0.clone()));
        }
        Ok(Box::new(RealKms::open_routed(
            fd,
            &self.handle,
            self.hub.clone(),
            self.mode,
        )?))
    }
}

/// Whether the DRM version ioctl on `fd` names the vkms driver (the same reading `--drm-node`
/// checks with [`require_vkms`]).
fn is_vkms(fd: &OwnedFd) -> Result<bool, KmsError> {
    let device = DrmDeviceFd::new(DeviceFd::from(fd.try_clone()?));
    let driver = device.get_driver()?;
    Ok(require_vkms(driver.name()).is_ok())
}
