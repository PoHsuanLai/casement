//! The KMS seam (P1-Z; filled by P1-D2): what a DRM device offers and how it is driven.
//!
//! [`Kms`] is the one trait the tty backend drives a display device through. The real
//! implementation is `kms_real.rs` over the device crates (feature `tty`); the fake is
//! `casement_testkit::fakes::FakeKms`, which replays recorded and synthetic topologies with a
//! pixman image per connector. A device is opened by a [`KmsOpener`] (the session hands the
//! file descriptor; the fake hands a fixture).
//!
//! ```no_run
//! use casement_backend::tty::Kms;
//!
//! fn lit_connectors(kms: &dyn Kms) -> usize {
//!     kms.resources().connectors.len()
//! }
//! ```

use super::surface::ScanoutSurface;
use super::udev::DevicePath;
use crate::headless::ConnectorEvent;
use casement_core::OutputName;
use casement_core::gpu::FormatEntry;
use casement_core::output::{
    GammaRamp, Mode, OutputConfig, OutputPower, PhysicalMm, VrrCapability,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A connector's kernel object id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConnectorId(pub u32);

/// A CRTC's kernel object id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CrtcId(pub u32);

/// A plane's kernel object id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PlaneId(pub u32);

/// What a plane is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PlaneKind {
    /// The plane the frame is scanned out from.
    Primary,
    /// An extra plane for direct scanout of a client buffer.
    Overlay,
    /// The hardware cursor plane.
    Cursor,
}

/// Whether a monitor is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ConnectorStatus {
    /// A monitor is attached.
    Connected,
    /// None is.
    Disconnected,
    /// The driver cannot tell.
    Unknown,
}

/// One connector and what its monitor reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ConnectorFacts {
    /// Its kernel id.
    pub id: ConnectorId,
    /// Its name (`DP-1`, `eDP-1`).
    pub name: OutputName,
    /// Whether a monitor is attached.
    pub status: ConnectorStatus,
    /// The modes the monitor offers.
    pub modes: Vec<Mode>,
    /// The monitor's physical size.
    pub physical: PhysicalMm,
    /// The raw EDID block (empty when unreadable); `connector::parse_edid` reads it.
    pub edid: Vec<u8>,
    /// Whether the monitor can vary its refresh rate.
    pub vrr: VrrCapability,
    /// The CRTCs that can drive it.
    pub possible_crtcs: Vec<CrtcId>,
}

impl ConnectorFacts {
    /// A connector with no modes, no EDID and no size, that cannot vary its refresh and can be
    /// driven by no CRTC yet.
    pub fn new(id: ConnectorId, name: OutputName, status: ConnectorStatus) -> ConnectorFacts {
        ConnectorFacts {
            id,
            name,
            status,
            modes: Vec::new(),
            physical: PhysicalMm { w: 0, h: 0 },
            edid: Vec::new(),
            vrr: VrrCapability::Incapable,
            possible_crtcs: Vec::new(),
        }
    }

    /// The same with these modes.
    #[must_use]
    pub fn with_modes(self, modes: Vec<Mode>) -> ConnectorFacts {
        ConnectorFacts { modes, ..self }
    }

    /// The same with this physical size.
    #[must_use]
    pub fn with_physical(self, physical: PhysicalMm) -> ConnectorFacts {
        ConnectorFacts { physical, ..self }
    }

    /// The same with this raw EDID.
    #[must_use]
    pub fn with_edid(self, edid: Vec<u8>) -> ConnectorFacts {
        ConnectorFacts { edid, ..self }
    }

    /// The same with this variable-refresh capability.
    #[must_use]
    pub fn with_vrr(self, vrr: VrrCapability) -> ConnectorFacts {
        ConnectorFacts { vrr, ..self }
    }

    /// The same with these CRTCs able to drive it.
    #[must_use]
    pub fn with_crtcs(self, possible_crtcs: Vec<CrtcId>) -> ConnectorFacts {
        ConnectorFacts {
            possible_crtcs,
            ..self
        }
    }
}

/// One CRTC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CrtcFacts {
    /// Its kernel id.
    pub id: CrtcId,
    /// Whether it can drive a variable-refresh monitor.
    pub vrr: VrrCapability,
    /// How many steps its gamma table has per channel (`GAMMA_LUT_SIZE`; 0 when it has no gamma
    /// table). A recorded fixture that predates the field reads as [`DEFAULT_GAMMA_SIZE`].
    #[serde(default = "default_gamma_size")]
    pub gamma_size: u32,
}

/// The gamma table size a CRTC is taken to have when nothing says (most hardware: 256).
pub const DEFAULT_GAMMA_SIZE: u32 = 256;

fn default_gamma_size() -> u32 {
    DEFAULT_GAMMA_SIZE
}

impl CrtcFacts {
    /// A CRTC with a [`DEFAULT_GAMMA_SIZE`] gamma table.
    pub fn new(id: CrtcId, vrr: VrrCapability) -> CrtcFacts {
        CrtcFacts {
            id,
            vrr,
            gamma_size: DEFAULT_GAMMA_SIZE,
        }
    }

    /// The same with a gamma table of `gamma_size` steps.
    #[must_use]
    pub fn with_gamma_size(self, gamma_size: u32) -> CrtcFacts {
        CrtcFacts { gamma_size, ..self }
    }
}

/// One plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PlaneFacts {
    /// Its kernel id.
    pub id: PlaneId,
    /// What it is for.
    pub kind: PlaneKind,
    /// The CRTCs it can sit on.
    pub possible_crtcs: Vec<CrtcId>,
    /// The formats and modifiers it scans out.
    pub formats: Vec<FormatEntry>,
}

impl PlaneFacts {
    /// A plane.
    pub fn new(
        id: PlaneId,
        kind: PlaneKind,
        possible_crtcs: Vec<CrtcId>,
        formats: Vec<FormatEntry>,
    ) -> PlaneFacts {
        PlaneFacts {
            id,
            kind,
            possible_crtcs,
            formats,
        }
    }
}

/// Everything a device offers: connectors, CRTCs and planes.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct KmsResources {
    /// The connectors.
    pub connectors: Vec<ConnectorFacts>,
    /// The CRTCs.
    pub crtcs: Vec<CrtcFacts>,
    /// The planes.
    pub planes: Vec<PlaneFacts>,
}

impl KmsResources {
    /// A device's resources.
    pub fn new(
        connectors: Vec<ConnectorFacts>,
        crtcs: Vec<CrtcFacts>,
        planes: Vec<PlaneFacts>,
    ) -> KmsResources {
        KmsResources {
            connectors,
            crtcs,
            planes,
        }
    }
}

/// Whether a variable-refresh property is set on an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VrrState {
    /// Fixed refresh.
    Off,
    /// Variable refresh.
    On,
}

/// Whether a commit may change modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ModesetPolicy {
    /// A full modeset is fine (start-up, a hotplug).
    Allow,
    /// Only a page flip (a frame): a commit that needs a modeset is `NeedsModeset`.
    Forbid,
}

/// One output's part of a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OutputCommit {
    /// The connector.
    pub connector: ConnectorId,
    /// The CRTC that drives it.
    pub crtc: CrtcId,
    /// Its mode.
    pub mode: Mode,
    /// Variable refresh on or off.
    pub vrr: VrrState,
}

impl OutputCommit {
    /// An output's commit.
    pub fn new(connector: ConnectorId, crtc: CrtcId, mode: Mode, vrr: VrrState) -> OutputCommit {
        OutputCommit {
            connector,
            crtc,
            mode,
            vrr,
        }
    }
}

/// One atomic commit: the outputs it sets up.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct KmsCommit {
    /// The outputs.
    pub outputs: Vec<OutputCommit>,
    /// Whether a modeset is allowed.
    pub modeset: ModesetPolicy,
}

impl KmsCommit {
    /// A commit.
    pub fn new(outputs: Vec<OutputCommit>, modeset: ModesetPolicy) -> KmsCommit {
        KmsCommit { outputs, modeset }
    }
}

/// What a `TEST_ONLY` commit said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommitVerdict {
    /// The commit would succeed.
    Accepted,
    /// It would succeed only as a full modeset.
    NeedsModeset,
    /// It would fail.
    Refused(KmsRefusal),
}

/// Why the kernel refuses a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum KmsRefusal {
    /// Not enough bandwidth for the modes together.
    #[error("not enough bandwidth")]
    Bandwidth,
    /// No CRTC is free for a connector.
    #[error("no free CRTC")]
    NoCrtc,
    /// The mode is not supported.
    #[error("mode not supported")]
    Mode,
    /// No plane arrangement fits.
    #[error("no plane fits")]
    Planes,
    /// The commit needs a modeset and the caller forbade one.
    #[error("a modeset is needed and was forbidden")]
    ModesetForbidden,
    /// Anything else the driver says no to.
    #[error("unsupported")]
    Unsupported,
}

/// Why a KMS call failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum KmsError {
    /// The device returned an I/O error.
    #[error("kms: {0}")]
    Io(#[from] std::io::Error),
    /// The connector is not on this device.
    #[error("unknown connector {0:?}")]
    UnknownConnector(ConnectorId),
    /// The commit was refused.
    #[error("commit refused: {0}")]
    Refused(#[from] KmsRefusal),
    /// The device went away (an unplugged eGPU, a revoked session).
    #[error("device {} is gone", .0.display())]
    DeviceGone(PathBuf),
    /// The device is a virtual test display (vkms), which only `--drm-node` drives.
    #[error("{} is a virtual test display; only --drm-node drives one", .0.display())]
    VirtualDisplay(PathBuf),
    /// No open device drives the output of this name.
    #[error("no device drives the output {0}")]
    NoDriver(String),
}

/// One display device, driven atomically.
pub trait Kms: std::fmt::Debug {
    /// What the device offers now.
    fn resources(&self) -> KmsResources;

    /// Checks `commit` without applying it (`TEST_ONLY`).
    fn test(&mut self, commit: &KmsCommit) -> CommitVerdict;

    /// Applies `commit`.
    fn commit(&mut self, commit: KmsCommit) -> Result<(), KmsError>;

    /// Lights or darkens one connector (DPMS).
    fn set_power(&mut self, connector: ConnectorId, power: OutputPower) -> Result<(), KmsError>;

    /// Loads a gamma ramp on one connector's CRTC (the plan's sketch has no gamma call;
    /// `Effect::SetGamma` needs one: FINDINGS).
    fn set_gamma(&mut self, connector: ConnectorId, ramp: &GammaRamp) -> Result<(), KmsError>;

    /// The connector changes since the last call (hotplug, a link retrain), as the same
    /// events the headless backend reports.
    fn events(&mut self) -> Vec<ConnectorEvent>;

    /// The surface that scans `connector` out, made once a commit has lit it (P1-T: the D2 to
    /// D3 hand-off; the real device gives up its `DrmSurface` and the vblank source). Asking
    /// again for a connector lit by a later commit gives a fresh surface.
    fn scanout(
        &mut self,
        connector: ConnectorId,
        config: &OutputConfig,
    ) -> Result<Box<dyn ScanoutSurface>, KmsError>;

    /// The seat was lent away (a VT switch, a suspend): give up master, touch nothing until
    /// [`resume`](Kms::resume).
    fn pause(&mut self);

    /// The seat is back: take master again. The caller then commits every output again.
    fn resume(&mut self) -> Result<(), KmsError>;
}

/// Opens a display device. Real: asks the session for the file descriptor and wraps it; fake:
/// hands out a fixture.
pub trait KmsOpener: std::fmt::Debug {
    /// The device at `path`, opened.
    fn open(&mut self, path: &DevicePath) -> Result<Box<dyn Kms>, KmsError>;
}
