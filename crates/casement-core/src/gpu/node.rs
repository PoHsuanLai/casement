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

    /// The node a Linux `dev_t` names (`st_rdev`, DRM's `dev_id`): glibc's `major`/`minor`
    /// split of the 64-bit number.
    ///
    /// ```
    /// use casement_core::gpu::DrmNodeId;
    ///
    /// assert_eq!(DrmNodeId::from_dev_t((226 << 8) | 128), DrmNodeId::new(226, 128));
    /// ```
    pub fn from_dev_t(dev: u64) -> DrmNodeId {
        let major = ((dev >> 32) & 0xffff_f000) | ((dev >> 8) & 0x0fff);
        let minor = ((dev >> 12) & 0xffff_ff00) | (dev & 0x00ff);
        DrmNodeId {
            major: u32::try_from(major).unwrap_or(u32::MAX),
            minor: u32::try_from(minor).unwrap_or(u32::MAX),
        }
    }

    /// The Linux `dev_t` of this node: the inverse of [`DrmNodeId::from_dev_t`].
    ///
    /// ```
    /// use casement_core::gpu::DrmNodeId;
    ///
    /// let node = DrmNodeId::new(226, 129);
    /// assert_eq!(DrmNodeId::from_dev_t(node.dev_t()), node);
    /// ```
    pub fn dev_t(&self) -> u64 {
        let (major, minor) = (u64::from(self.major), u64::from(self.minor));
        ((major & 0xffff_f000) << 32)
            | ((major & 0x0fff) << 8)
            | ((minor & 0xffff_ff00) << 12)
            | (minor & 0x00ff)
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
        let bytes = text.as_bytes();
        let lower_hex = |slice: &[u8]| {
            slice
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        };
        // dddd:bb:ss.f
        let well_formed = bytes.len() == 12
            && lower_hex(&bytes[0..4])
            && bytes[4] == b':'
            && lower_hex(&bytes[5..7])
            && bytes[7] == b':'
            && lower_hex(&bytes[8..10])
            && bytes[10] == b'.'
            && (b'0'..=b'7').contains(&bytes[11]);
        if well_formed {
            Ok(ByPathId(text.to_owned()))
        } else {
            Err(CoreError::Malformed {
                what: "PCI address",
                text: text.to_owned(),
            })
        }
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
