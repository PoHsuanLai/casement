//! A fabricated `/sys` and `/dev` for GPU detection (A6, P1-Z; filled by P1-D4).
//!
//! The real adapter (`casement_backend::gpu::SysRenderNodes`) reads GPUs from an injected
//! [`SysRoot`]. In the jail that root can be the real one with `jail.sh --fabricate-gpus N`
//! (A3); in process, [`FabricatedSysRoot::build`] writes the same tree from a [`GpuTopology`]
//! into a directory a test owns, so detection, the pure plan and the soft multi-GPU stack all
//! run from one description.

use super::GpuTopology;
use casement_backend::gpu::SysRoot;
use std::path::Path;

/// Builds fabricated sys roots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FabricatedSysRoot;

impl FabricatedSysRoot {
    /// Writes `topology` under `dir`: `sys/dev/char/226:(128+id)/device/{vendor,boot_vga}`,
    /// the connectors each GPU drives, and `dev/dri/by-path/…-render` links; returns the root.
    pub fn build(topology: &GpuTopology, dir: &Path) -> Result<SysRoot, std::io::Error> {
        let _ = (topology, dir);
        todo!("P1-D4: FabricatedSysRoot::build")
    }
}
