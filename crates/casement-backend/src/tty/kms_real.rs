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

use super::hotplug::{KnownConnectors, reconcile};
use super::judge::{Lit, judge};
use super::kms::{
    CommitVerdict, ConnectorFacts, ConnectorId, ConnectorStatus, CrtcFacts, CrtcId, Kms, KmsCommit,
    KmsError, KmsOpener, KmsRefusal, KmsResources, OutputCommit, PlaneFacts, PlaneId, PlaneKind,
    VrrState,
};
use super::surface::ScanoutSurface;
use super::surface_real::{Flips, RealScanout};
use super::udev::DevicePath;
use crate::headless::ConnectorEvent;
use casement_core::OutputName;
use casement_core::geometry::PhysicalSize;
use casement_core::gpu::{FormatEntry, Fourcc, Modifier};
use casement_core::output::{
    GammaRamp, Mode, ModePreference, OutputConfig, OutputPower, PhysicalMm, VrrCapability,
};
use casement_core::time::Millihertz;
use smithay::backend::drm::{DrmDevice, DrmDeviceFd, DrmEvent, DrmSurface};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::reexports::calloop::LoopHandle;
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
    gbm: GbmDevice<DrmDeviceFd>,
    flips: Rc<RefCell<Flips>>,
    /// The device's renderer, made for the first scanout and shared by the rest.
    renderer: Option<Rc<RefCell<GlesRenderer>>>,
    /// The surfaces lit and not yet handed to a scanout.
    surfaces: BTreeMap<ConnectorId, DrmSurface>,
    /// The connectors whose surface is out with a scanout (gone when the scanout is dropped).
    out: BTreeMap<ConnectorId, Weak<()>>,
    lit: Lit,
    power: BTreeMap<ConnectorId, OutputPower>,
    known: KnownConnectors,
    last: KmsResources,
}

fn io_error(error: impl std::error::Error + Send + Sync + 'static) -> KmsError {
    KmsError::Io(io::Error::other(error))
}

impl RealKms {
    /// Wraps the device behind `fd`: reads its resources, takes master, and has `handle`'s loop
    /// read the device's vblank events.
    pub fn open<D: 'static>(
        fd: OwnedFd,
        handle: &LoopHandle<'static, D>,
    ) -> Result<RealKms, KmsError> {
        let fd = DrmDeviceFd::new(DeviceFd::from(fd));
        let (device, notifier) = DrmDevice::new(fd.clone(), false).map_err(io_error)?;
        let gbm = GbmDevice::new(fd)?;
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
            gbm,
            flips,
            renderer: None,
            surfaces: BTreeMap::new(),
            out: BTreeMap::new(),
            lit: Lit::new(),
            power: BTreeMap::new(),
            known,
            last,
        })
    }

    /// The device's GLES renderer, made on first use.
    fn renderer(&mut self) -> Result<Rc<RefCell<GlesRenderer>>, KmsError> {
        if let Some(renderer) = &self.renderer {
            return Ok(Rc::clone(renderer));
        }
        let renderer = casement_render::gles_on(self.gbm.clone())
            .map_err(|error| KmsError::Io(io::Error::other(error.to_string())))?;
        let shared = Rc::new(RefCell::new(renderer));
        self.renderer = Some(Rc::clone(&shared));
        Ok(shared)
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
            surface
                .use_vrr(output.vrr == VrrState::On)
                .map_err(io_error)?;
        } else {
            let surface = self
                .device
                .create_surface(crtc, raw, &[connector])
                .map_err(io_error)?;
            surface
                .use_vrr(output.vrr == VrrState::On)
                .map_err(io_error)?;
            self.surfaces.insert(output.connector, surface);
        }
        Ok(())
    }
}

impl Kms for RealKms {
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
        let surface = self
            .surfaces
            .remove(&connector)
            .ok_or(KmsError::UnknownConnector(connector))?;
        let renderer = self.renderer()?;
        let claim = Rc::new(());
        self.out.insert(connector, Rc::downgrade(&claim));
        let scanout = RealScanout::open(
            connector,
            surface,
            &self.gbm,
            renderer,
            self.device.cursor_size(),
            Rc::clone(&self.flips),
            claim,
            config,
        )?;
        Ok(Box::new(scanout))
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
}

impl<D: 'static> RealKmsOpener<D> {
    /// An opener that gets descriptors from `fds` and has `handle`'s loop read each device.
    pub fn new(fds: Box<dyn FdSource>, handle: LoopHandle<'static, D>) -> RealKmsOpener<D> {
        RealKmsOpener { fds, handle }
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
        Ok(Box::new(RealKms::open(fd, &self.handle)?))
    }
}
