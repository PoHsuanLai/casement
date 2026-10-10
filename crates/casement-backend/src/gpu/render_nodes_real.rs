//! Real GPU detection (feature `tty`): `RenderNodes` over `/sys` and `/dev` under an injected
//! [`SysRoot`], a thin adapter (B7). P1-Z declares it; P1-D4 fills it.

use super::{GpuError, GpuHandle, RenderNodes, SysRoot};
use casement_core::gpu::{DrmNodeId, GpuFacts};

/// Detection over a sys root.
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
        todo!("P1-D4: SysRenderNodes::nodes")
    }

    fn open(&mut self, node: &DrmNodeId) -> Result<GpuHandle, GpuError> {
        let _ = node;
        todo!("P1-D4: SysRenderNodes::open")
    }
}
