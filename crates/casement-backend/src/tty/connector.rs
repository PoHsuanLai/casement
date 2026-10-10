//! A KMS connector as casement's output facts (P1-Z; filled by P1-D2): the EDID reader, the
//! facts of an attached connector, and the pure planning of what to commit for it (mode,
//! CRTC, variable refresh).

use super::kms::{
    ConnectorFacts, ConnectorStatus, CrtcFacts, CrtcId, KmsResources, OutputCommit, VrrState,
};
use casement_core::OutputKey;
use casement_core::OutputName;
use casement_core::output::{
    EdidFacts, EdidSerial, ModeWish, OutputFacts, VrrCapability, pick_mode,
};
use std::collections::{BTreeMap, BTreeSet};

/// The size of an EDID base block.
const BASE_BLOCK: usize = 128;
/// The fixed first eight bytes of an EDID.
const HEADER: [u8; 8] = [0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00];
/// Where the four 18-byte descriptors start, and how long each is.
const DESCRIPTORS: [usize; 4] = [54, 72, 90, 108];
const DESCRIPTOR_LEN: usize = 18;
/// Descriptor tags: the monitor name and the serial text.
const TAG_NAME: u8 = 0xfc;
const TAG_SERIAL: u8 = 0xff;

/// Why an EDID block could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EdidError {
    /// The block is shorter than the 128 bytes the base block takes.
    #[error("EDID block too short: {0} bytes")]
    TooShort(usize),
    /// The fixed header is wrong.
    #[error("EDID header is wrong")]
    BadHeader,
    /// The checksum does not add up.
    #[error("EDID checksum is wrong")]
    BadChecksum,
}

/// The make, model and serial an EDID base block reports. The make is the three-letter
/// manufacturer id, the model the monitor-name descriptor (the product code in hex when there is
/// none) and the serial the serial-text descriptor, else the numeric serial when it is not zero.
///
/// ```
/// use casement_backend::tty::parse_edid;
///
/// assert!(parse_edid(&[]).is_err());
/// ```
pub fn parse_edid(bytes: &[u8]) -> Result<EdidFacts, EdidError> {
    let block = bytes
        .get(..BASE_BLOCK)
        .ok_or(EdidError::TooShort(bytes.len()))?;
    if block[..8] != HEADER {
        return Err(EdidError::BadHeader);
    }
    let sum = block.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    if sum != 0 {
        return Err(EdidError::BadChecksum);
    }
    let model = descriptor_text(block, TAG_NAME)
        .unwrap_or_else(|| format!("{:04X}", u16::from_le_bytes([block[10], block[11]])));
    let numeric = u32::from_le_bytes([block[12], block[13], block[14], block[15]]);
    let serial = match (descriptor_text(block, TAG_SERIAL), numeric) {
        (Some(text), _) => EdidSerial::Known(text),
        (None, 0) => EdidSerial::Unknown,
        (None, number) => EdidSerial::Known(number.to_string()),
    };
    Ok(EdidFacts {
        make: manufacturer(block[8], block[9]),
        model,
        serial,
    })
}

/// The three letters packed in two big-endian bytes, five bits each, `A` being 1.
fn manufacturer(high: u8, low: u8) -> String {
    let packed = u16::from_be_bytes([high, low]);
    [10u16, 5, 0]
        .iter()
        .map(|shift| char::from(((packed >> shift) & 0x1f) as u8 + b'A' - 1))
        .collect()
}

/// The text of the descriptor tagged `tag`, trimmed, when the block has one.
fn descriptor_text(block: &[u8], tag: u8) -> Option<String> {
    DESCRIPTORS
        .iter()
        .map(|start| &block[*start..*start + DESCRIPTOR_LEN])
        .find(|d| d[..3] == [0, 0, 0] && d[3] == tag)
        .map(|d| {
            let text: String = d[5..]
                .iter()
                .take_while(|byte| **byte != b'\n')
                .map(|byte| char::from(*byte))
                .collect();
            text.trim().to_owned()
        })
        .filter(|text| !text.is_empty())
}

/// The facts of an attached connector: its identity from the EDID (an unreadable one gives an
/// anonymous display keyed by its connector), its modes, physical size and variable-refresh
/// capability.
///
/// ```
/// use casement_backend::tty::{ConnectorFacts, ConnectorId, ConnectorStatus, output_facts};
/// use casement_core::OutputName;
///
/// let connector = ConnectorFacts::new(
///     ConnectorId(1),
///     OutputName::parse("DP-1").unwrap(),
///     ConnectorStatus::Connected,
/// );
/// assert_eq!(output_facts(&connector).key.as_str(), "connector:DP-1");
/// ```
pub fn output_facts(connector: &ConnectorFacts) -> OutputFacts {
    let edid = parse_edid(&connector.edid).unwrap_or_else(|_| EdidFacts {
        make: "Unknown".to_owned(),
        model: "Unknown".to_owned(),
        serial: EdidSerial::Unknown,
    });
    OutputFacts {
        name: connector.name.clone(),
        key: OutputKey::of(&edid, &connector.name),
        edid,
        modes: connector.modes.clone(),
        physical: connector.physical,
        vrr: connector.vrr,
    }
}

/// The variable-refresh policy: whether to ask for it where it is possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum VrrPolicy {
    /// Fixed refresh everywhere.
    #[default]
    Off,
    /// Variable refresh on every output whose monitor and CRTC both support it.
    WhenCapable,
}

/// The variable-refresh state `policy` gives an output: on only when asked for and when both
/// the monitor and the CRTC can do it, so a commit never sets a property the device lacks.
pub fn vrr_state(policy: VrrPolicy, connector: &ConnectorFacts, crtc: &CrtcFacts) -> VrrState {
    match (policy, connector.vrr, crtc.vrr) {
        (VrrPolicy::WhenCapable, VrrCapability::Capable, VrrCapability::Capable) => VrrState::On,
        _ => VrrState::Off,
    }
}

/// What to commit for every attached connector of `resources`: the mode `wishes` asks for by
/// connector name (the preferred mode otherwise), on the first CRTC that can drive it and is
/// not taken, with variable refresh per `policy`. A connector with no mode is left dark, and
/// so is one no CRTC is left for.
pub fn plan_outputs(
    resources: &KmsResources,
    wishes: &BTreeMap<OutputName, ModeWish>,
    policy: VrrPolicy,
) -> Vec<OutputCommit> {
    let mut taken: BTreeSet<CrtcId> = BTreeSet::new();
    let mut plan = Vec::new();
    let attached = resources
        .connectors
        .iter()
        .filter(|c| c.status == ConnectorStatus::Connected);
    for connector in attached {
        let wish = wishes
            .get(&connector.name)
            .copied()
            .unwrap_or(ModeWish::Preferred);
        let mode = pick_mode(&output_facts(connector), &wish);
        let crtc = connector
            .possible_crtcs
            .iter()
            .find(|id| !taken.contains(id))
            .and_then(|id| resources.crtcs.iter().find(|c| c.id == *id));
        if let (Some(crtc), true) = (crtc, mode.size.w > 0 && mode.size.h > 0) {
            taken.insert(crtc.id);
            let vrr = vrr_state(policy, connector, crtc);
            plan.push(OutputCommit::new(connector.id, crtc.id, mode, vrr));
        }
    }
    plan
}
