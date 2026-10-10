//! A fabricated `/sys` and `/dev` for GPU detection (A6, P1-Z; filled by P1-D4).
//!
//! The real adapter (`casement_backend::gpu::SysRenderNodes`) reads GPUs from an injected
//! [`SysRoot`]. In the jail that root can be the real one with `jail.sh --fabricate-gpus N`
//! (A3); in process, [`FabricatedSysRoot::build`] writes the same tree from a [`GpuTopology`]
//! into a directory a test owns, so detection, the pure plan and the soft multi-GPU stack all
//! run from one description.

use super::GpuTopology;
use casement_backend::gpu::SysRoot;
use std::io;
use std::path::Path;

/// Builds fabricated sys roots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FabricatedSysRoot;

impl FabricatedSysRoot {
    /// Writes `topology` under `dir`: `sys/dev/char/226:(128+id)/device/{vendor,boot_vga}`,
    /// the connectors each GPU drives (`device/drm/card<id>/card<id>-<NAME>/status`), and
    /// `dev/dri/renderD<minor>` with its `dev/dri/by-path/pci-<address>-render` link; returns
    /// the root.
    ///
    /// ```
    /// use casement_backend::gpu::{RenderNodes, SysRenderNodes};
    /// use casement_testkit::gpu::{FabricatedSysRoot, GpuTopology};
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let root = FabricatedSysRoot::build(&GpuTopology::single(), dir.path()).unwrap();
    /// assert_eq!(SysRenderNodes::new(root).nodes().len(), 1);
    /// ```
    pub fn build(topology: &GpuTopology, dir: &Path) -> Result<SysRoot, io::Error> {
        for (index, gpu) in topology.gpus.iter().enumerate() {
            let node = gpu.node();
            let device = dir
                .join("sys/dev/char")
                .join(format!("{}:{}", node.major, node.minor))
                .join("device");
            std::fs::create_dir_all(&device)?;
            std::fs::write(
                device.join("vendor"),
                format!("{}\n", vendor_text(gpu.vendor)),
            )?;
            let boot = if index == 0 { "1\n" } else { "0\n" };
            std::fs::write(device.join("boot_vga"), boot)?;
            let card = format!("card{}", gpu.id.0);
            for output in &gpu.drives {
                let connector = device
                    .join("drm")
                    .join(&card)
                    .join(format!("{card}-{}", output.as_str()));
                std::fs::create_dir_all(&connector)?;
                std::fs::write(connector.join("status"), "connected\n")?;
            }
            std::fs::create_dir_all(device.join("drm").join(&card))?;
            let by_path = dir.join("dev/dri/by-path");
            std::fs::create_dir_all(&by_path)?;
            std::fs::write(
                dir.join("dev/dri").join(format!("renderD{}", node.minor)),
                b"",
            )?;
            let address = gpu.by_path().map_err(io::Error::other)?;
            std::os::unix::fs::symlink(
                format!("../renderD{}", node.minor),
                by_path.join(format!("pci-{}-render", address.as_str())),
            )?;
        }
        Ok(SysRoot::under(dir))
    }
}

/// The text sysfs holds for a vendor: `0x1002`.
fn vendor_text(vendor: casement_core::gpu::VendorFact) -> String {
    use casement_core::gpu::VendorFact;
    let id = match vendor {
        VendorFact::Amd => 0x1002,
        VendorFact::Nvidia => 0x10de,
        VendorFact::Intel => 0x8086,
        VendorFact::Other(id) => id,
        // A vendor core learns later: unreadable to detection, which then reads it as 0.
        _ => 0,
    };
    format!("{id:#06x}")
}
