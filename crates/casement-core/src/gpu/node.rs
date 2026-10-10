//! What detection reports about one GPU.

use crate::error::CoreError;
use crate::ids::OutputName;
use serde::{Deserialize, Serialize};

/// A DRM node by device number (`226:128` is `/dev/dri/renderD128`): what the kernel calls it,
/// not a path, so a fabricated tree and a real one name nodes the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DrmNodeId {
    /// The device major (226 for DRM).
    pub major: u32,
    /// The device minor (the node: 0..63 primary, 128..191 render).
    pub minor: u32,
}

impl DrmNodeId {
    /// The node `major:minor`.
    pub fn new(major: u32, minor: u32) -> DrmNodeId {
        DrmNodeId { major, minor }
    }
}

/// A GPU's stable PCI identity, the tail of `/dev/dri/by-path/pci-0000:01:00.0-render` without
/// the `pci-` and the suffix (`0000:01:00.0`): what the `gpu.render` setting names, because
/// card numbers move between boots and PCI addresses do not.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ByPathId(String);

impl ByPathId {
    /// The id written as `text`: a PCI address `dddd:bb:ss.f` in lowercase hex.
    pub fn parse(text: &str) -> Result<ByPathId, CoreError> {
        let _ = text;
        todo!("P1-D4: ByPathId::parse")
    }

    /// The id's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ByPathId {
    type Error = CoreError;
    fn try_from(text: String) -> Result<ByPathId, CoreError> {
        ByPathId::parse(&text)
    }
}

impl From<ByPathId> for String {
    fn from(id: ByPathId) -> String {
        id.0
    }
}

/// The GPU's vendor, as far as the plan cares (quirks are keyed by it, QUIRKS.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum VendorFact {
    /// AMD (PCI vendor `0x1002`).
    Amd,
    /// NVIDIA (`0x10de`).
    Nvidia,
    /// Intel (`0x8086`).
    Intel,
    /// Any other PCI vendor id.
    Other(u16),
}

impl VendorFact {
    /// The vendor a PCI vendor id names.
    pub fn from_pci_vendor(id: u16) -> VendorFact {
        match id {
            0x1002 => VendorFact::Amd,
            0x10de => VendorFact::Nvidia,
            0x8086 => VendorFact::Intel,
            other => VendorFact::Other(other),
        }
    }
}

/// Whether the firmware booted on this GPU (`boot_vga` in sysfs): the tie-break when several
/// GPUs drive outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BootVga {
    /// The firmware's display GPU.
    Primary,
    /// Any other.
    Secondary,
}

/// Everything detection reports about one GPU.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GpuFacts {
    /// Its render node.
    pub node: DrmNodeId,
    /// Its vendor.
    pub vendor: VendorFact,
    /// Whether the firmware booted on it.
    pub boot_vga: BootVga,
    /// The connectors it drives (the ones with a monitor attached, by name).
    pub drives: Vec<OutputName>,
    /// Its PCI identity.
    pub by_path: ByPathId,
}

impl GpuFacts {
    /// A GPU's facts.
    pub fn new(
        node: DrmNodeId,
        vendor: VendorFact,
        boot_vga: BootVga,
        drives: Vec<OutputName>,
        by_path: ByPathId,
    ) -> GpuFacts {
        GpuFacts {
            node,
            vendor,
            boot_vga,
            drives,
            by_path,
        }
    }
}
