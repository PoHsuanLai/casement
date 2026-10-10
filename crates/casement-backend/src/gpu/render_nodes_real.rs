//! GPU detection over `/sys` and `/dev` under an injected [`SysRoot`] (B7, A6). It only reads
//! files, so it needs no device crate and builds without the `tty` feature: the test kit runs
//! it over a fabricated tree and the compositor over the host's. P1-Z declared it; P1-D4
//! filled it.
//!
//! The tree it reads (the kernel's own layout, so a fabricated tree is the same shape):
//!
//! * `sys/dev/char/226:<minor>/`: one directory or link per DRM node; render nodes are minors
//!   128 to 191 (the drm crate types `192..` as nothing, so a fabricated node never sits there).
//! * `sys/dev/char/226:<minor>/device/vendor` (`0x1002`) and `.../boot_vga` (`1`).
//! * `sys/dev/char/226:<minor>/device/drm/card<N>/card<N>-<CONNECTOR>/status`, `connected`
//!   when a monitor is attached.
//! * `dev/dri/by-path/pci-<address>-render`, a link to `renderD<minor>`: the GPU's PCI identity,
//!   which is what the `gpu.render` setting names. A GPU with no such link falls back to the
//!   file name of its `device` link when that is a PCI address, and is skipped otherwise.

use super::{GpuError, GpuHandle, RenderNodes, SysRoot};
use casement_core::OutputName;
use casement_core::gpu::{BootVga, ByPathId, DrmNodeId, GpuFacts, VendorFact};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::path::Path;

/// The DRM character-device major.
const DRM_MAJOR: u32 = 226;

/// The render-node minors: `renderD128` to `renderD191`.
const RENDER_MINORS: std::ops::Range<u32> = 128..192;

/// Detection over a sys root.
///
/// ```
/// use casement_backend::gpu::{RenderNodes, SysRenderNodes, SysRoot};
///
/// let nowhere = std::path::Path::new("/nonexistent-sys-root");
/// let nodes = SysRenderNodes::new(SysRoot::under(nowhere));
/// assert!(nodes.nodes().is_empty(), "no tree, no GPU");
/// ```
#[derive(Debug)]
pub struct SysRenderNodes {
    /// Where `/sys` and `/dev` are.
    pub root: SysRoot,
}

impl SysRenderNodes {
    /// Detection under `root`.
    pub fn new(root: SysRoot) -> SysRenderNodes {
        SysRenderNodes { root }
    }
}

impl RenderNodes for SysRenderNodes {
    fn nodes(&self) -> Vec<GpuFacts> {
        let links = by_path_links(self.root.dev());
        render_minors(self.root.sys())
            .into_iter()
            .filter_map(|minor| read_gpu(self.root.sys(), minor, &links))
            .collect()
    }

    fn open(&mut self, node: &DrmNodeId) -> Result<GpuHandle, GpuError> {
        if !self.nodes().iter().any(|gpu| gpu.node == *node) {
            return Err(GpuError::UnknownNode(*node));
        }
        let path = self
            .root
            .dev()
            .join("dri")
            .join(format!("renderD{}", node.minor));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|source| GpuError::Open {
                node: *node,
                source,
            })?;
        Ok(GpuHandle::new(*node, file.into()))
    }
}

/// The render-range DRM minors under `sys/dev/char`, ascending.
fn render_minors(sys: &Path) -> Vec<u32> {
    let mut minors: Vec<u32> = std::fs::read_dir(sys.join("dev/char"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let (major, minor) = name.split_once(':')?;
            let (major, minor) = (major.parse::<u32>().ok()?, minor.parse::<u32>().ok()?);
            (major == DRM_MAJOR && RENDER_MINORS.contains(&minor)).then_some(minor)
        })
        .collect();
    minors.sort_unstable();
    minors
}

/// The PCI identity of each render minor, from `dev/dri/by-path/pci-<address>-render`.
fn by_path_links(dev: &Path) -> BTreeMap<u32, ByPathId> {
    std::fs::read_dir(dev.join("dri/by-path"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let address = name.strip_prefix("pci-")?.strip_suffix("-render")?;
            let id = ByPathId::parse(address).ok()?;
            let target = std::fs::read_link(entry.path()).ok()?;
            let node = target.file_name()?.to_str()?;
            let minor = node.strip_prefix("renderD")?.parse::<u32>().ok()?;
            Some((minor, id))
        })
        .collect()
}

/// What the tree says about one render minor; none when it is not a usable GPU.
fn read_gpu(sys: &Path, minor: u32, links: &BTreeMap<u32, ByPathId>) -> Option<GpuFacts> {
    let device = sys
        .join("dev/char")
        .join(format!("{DRM_MAJOR}:{minor}"))
        .join("device");
    let vendor = read_text(&device.join("vendor"))?;
    let vendor = u16::from_str_radix(vendor.trim().trim_start_matches("0x"), 16).ok()?;
    let boot_vga = match read_text(&device.join("boot_vga"))
        .as_deref()
        .map(str::trim)
    {
        Some("1") => BootVga::Primary,
        _ => BootVga::Secondary,
    };
    let by_path = links.get(&minor).cloned().or_else(|| {
        let target = std::fs::read_link(&device).ok()?;
        ByPathId::parse(target.file_name()?.to_str()?).ok()
    })?;
    Some(GpuFacts::new(
        DrmNodeId::new(DRM_MAJOR, minor),
        VendorFact::from_pci_vendor(vendor),
        boot_vga,
        connected_outputs(&device.join("drm")),
        by_path,
    ))
}

/// The connectors with a monitor attached under `device/drm`: `card<N>/card<N>-<NAME>/status`
/// reading `connected`. Sorted by name.
fn connected_outputs(drm: &Path) -> Vec<OutputName> {
    let mut names: Vec<OutputName> = std::fs::read_dir(drm)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|card| is_card_dir(&card.file_name().to_string_lossy()))
        .flat_map(|card| {
            let prefix = format!("{}-", card.file_name().to_string_lossy());
            std::fs::read_dir(card.path())
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(move |connector| {
                    let file = connector.file_name().into_string().ok()?;
                    let name = file.strip_prefix(&prefix)?.to_owned();
                    let status = read_text(&connector.path().join("status"))?;
                    (status.trim() == "connected")
                        .then(|| OutputName::parse(&name).ok())
                        .flatten()
                })
        })
        .collect();
    names.sort();
    names
}

/// Whether `name` is `card` and digits.
fn is_card_dir(name: &str) -> bool {
    name.strip_prefix("card").is_some_and(|digits| {
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn read_text(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}
