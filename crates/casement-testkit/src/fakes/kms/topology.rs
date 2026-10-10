//! Fake topologies: recorded fixtures and synthetic ones, as `KmsResources`.

use super::fake::LinkBehaviour;
use casement_backend::tty::{
    ConnectorFacts, ConnectorId, ConnectorStatus, CrtcFacts, CrtcId, KmsResources, PlaneFacts,
    PlaneId, PlaneKind,
};
use casement_core::OutputName;
use casement_core::geometry::PhysicalSize;
use casement_core::gpu::{FormatEntry, Fourcc, Modifier};
use casement_core::output::{Mode, ModePreference, PhysicalMm, VrrCapability};
use casement_core::time::Millihertz;
use std::path::Path;

/// Why a fixture could not be loaded.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum KmsFixtureError {
    /// The file could not be read.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file.
        path: std::path::PathBuf,
        /// Why.
        #[source]
        source: std::io::Error,
    },
    /// The file is not a recorded topology.
    #[error("{}: {source}", path.display())]
    Parse {
        /// The file.
        path: std::path::PathBuf,
        /// Why.
        #[source]
        source: serde_json::Error,
    },
}

/// The recorded topology in `path` (JSON of `KmsResources`).
pub fn load_fixture(path: &Path) -> Result<KmsResources, KmsFixtureError> {
    let text = std::fs::read_to_string(path).map_err(|source| KmsFixtureError::Io {
        path: path.to_owned(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| KmsFixtureError::Parse {
        path: path.to_owned(),
        source,
    })
}

/// Builders for the synthetic topologies the FakeKms suite runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SyntheticTopology {
    /// One laptop panel, `eDP-1`, 2880x1800 at 120 Hz, variable refresh capable.
    LaptopPanel,
    /// A panel and two external monitors on a dock.
    Dock,
    /// One monitor whose link retrains: it reports `Changed` after every commit.
    RetrainingMonitor,
    /// A connected connector that offers no mode at all.
    NoModes,
}

impl SyntheticTopology {
    /// The resources the topology offers.
    ///
    /// ```
    /// use casement_testkit::fakes::SyntheticTopology;
    ///
    /// assert_eq!(SyntheticTopology::Dock.resources().connectors.len(), 3);
    /// ```
    pub fn resources(self) -> KmsResources {
        match self {
            SyntheticTopology::LaptopPanel => device(vec![panel(1)], 1),
            SyntheticTopology::Dock => device(vec![panel(1), dell(2), hdmi(3)], 3),
            SyntheticTopology::RetrainingMonitor => device(vec![retrainer(1)], 1),
            SyntheticTopology::NoModes => device(vec![blind(1)], 1),
        }
    }

    /// How the topology's links behave after a commit.
    pub fn link(self) -> LinkBehaviour {
        match self {
            SyntheticTopology::RetrainingMonitor => LinkBehaviour::Retrains,
            _ => LinkBehaviour::Stable,
        }
    }
}

const CRTC_BASE: u32 = 10;
const PLANE_BASE: u32 = 20;
const XRGB8888: Fourcc = Fourcc(0x3432_5258);
const LINEAR: Modifier = Modifier(0);

fn crtc_ids(count: u32) -> Vec<CrtcId> {
    (0..count).map(|n| CrtcId(CRTC_BASE + n)).collect()
}

/// A device with `crtcs` CRTCs, each with a primary, an overlay and a cursor plane.
fn device(connectors: Vec<ConnectorFacts>, crtcs: u32) -> KmsResources {
    let ids = crtc_ids(crtcs);
    let all = ids.clone();
    let planes = ids
        .iter()
        .zip(0u32..)
        .flat_map(|(crtc, n)| {
            [PlaneKind::Primary, PlaneKind::Overlay, PlaneKind::Cursor]
                .into_iter()
                .zip(0u32..)
                .map(move |(kind, k)| {
                    let formats = vec![FormatEntry {
                        format: XRGB8888,
                        modifier: LINEAR,
                    }];
                    PlaneFacts::new(PlaneId(PLANE_BASE + n * 3 + k), kind, vec![*crtc], formats)
                })
        })
        .collect();
    let crtcs = ids
        .into_iter()
        .map(|id| CrtcFacts::new(id, VrrCapability::Capable))
        .collect();
    let connectors = connectors
        .into_iter()
        .map(|c| c.with_crtcs(all.clone()))
        .collect();
    KmsResources::new(connectors, crtcs, planes)
}

fn mode(w: u32, h: u32, hz: u32, preference: ModePreference) -> Mode {
    Mode {
        size: PhysicalSize { w, h },
        refresh: Millihertz(hz * 1000),
        preference,
    }
}

fn connector(id: u32, name: &str) -> ConnectorFacts {
    let name = OutputName::parse(name).unwrap_or_else(|_| unreachable!("static name"));
    ConnectorFacts::new(ConnectorId(id), name, ConnectorStatus::Connected)
}

fn panel(id: u32) -> ConnectorFacts {
    connector(id, "eDP-1")
        .with_modes(vec![
            mode(2880, 1800, 120, ModePreference::Preferred),
            mode(2880, 1800, 60, ModePreference::Other),
            mode(1920, 1200, 60, ModePreference::Other),
        ])
        .with_physical(PhysicalMm { w: 300, h: 190 })
        .with_edid(edid("BOE", "NE135A1M", None))
        .with_vrr(VrrCapability::Capable)
}

fn dell(id: u32) -> ConnectorFacts {
    connector(id, "DP-1")
        .with_modes(vec![
            mode(3840, 2160, 60, ModePreference::Preferred),
            mode(2560, 1440, 144, ModePreference::Other),
            mode(2560, 1440, 60, ModePreference::Other),
            mode(1920, 1080, 60, ModePreference::Other),
        ])
        .with_physical(PhysicalMm { w: 600, h: 340 })
        .with_edid(edid("DEL", "DELL U2723QE", Some("7GXK123")))
}

fn hdmi(id: u32) -> ConnectorFacts {
    connector(id, "HDMI-A-1")
        .with_modes(vec![
            mode(1920, 1080, 60, ModePreference::Preferred),
            mode(1280, 720, 60, ModePreference::Other),
        ])
        .with_physical(PhysicalMm { w: 510, h: 290 })
        .with_edid(edid("GSM", "LG FULL HD", None))
}

fn retrainer(id: u32) -> ConnectorFacts {
    connector(id, "DP-1")
        .with_modes(vec![
            mode(3840, 2160, 60, ModePreference::Preferred),
            mode(3840, 2160, 30, ModePreference::Other),
            mode(1920, 1080, 60, ModePreference::Other),
        ])
        .with_physical(PhysicalMm { w: 600, h: 340 })
        .with_edid(edid("DEL", "DELL U2723QE", Some("7GXK456")))
}

fn blind(id: u32) -> ConnectorFacts {
    connector(id, "DP-1")
}

/// An EDID base block: header, the three-letter `make`, the monitor `name` and an optional
/// serial text descriptor, with the checksum that makes the block sum to zero.
pub fn edid(make: &str, name: &str, serial: Option<&str>) -> Vec<u8> {
    let mut block = vec![0u8; 128];
    block[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
    let packed = make.bytes().take(3).fold(0u16, |acc, letter| {
        (acc << 5) | u16::from(letter - b'A' + 1)
    });
    block[8..10].copy_from_slice(&packed.to_be_bytes());
    block[10..12].copy_from_slice(&0x1234u16.to_le_bytes());
    block[18] = 1;
    block[19] = 4;
    descriptor(&mut block[54..72], 0xfc, name);
    if let Some(serial) = serial {
        descriptor(&mut block[72..90], 0xff, serial);
    }
    let sum = block[..127]
        .iter()
        .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    block[127] = 0u8.wrapping_sub(sum);
    block
}

fn descriptor(slot: &mut [u8], tag: u8, text: &str) {
    slot[3] = tag;
    let body = &mut slot[5..18];
    body.fill(b' ');
    let bytes = text.as_bytes();
    let used = bytes.len().min(12);
    body[..used].copy_from_slice(&bytes[..used]);
    body[used] = b'\n';
}
