//! The multi-GPU stack (P1-Z; filled by P1-D4): the render plan, and the renderers behind it.
//!
//! One GPU renders, each output is scanned out by the GPU that drives it, and a frame rendered
//! elsewhere is copied across (SP4). The copy machinery is smithay's `GpuManager`; the soft
//! topology tests run it over `SoftGraphicsApi` in the testkit.

use super::{GpuError, RenderNodes};
use casement_core::gpu::{DrmNodeId, GpuChoice, RenderPlan};

/// The plan and the GPUs it names.
#[derive(Debug)]
pub struct GpuStack {
    /// Who renders and who scans out.
    pub plan: RenderPlan,
}

impl GpuStack {
    /// Detects the GPUs on `nodes`, makes the plan under `choice` and opens what it names.
    pub fn start(nodes: &mut dyn RenderNodes, choice: &GpuChoice) -> Result<GpuStack, GpuError> {
        let _ = (nodes, choice);
        todo!("P1-D4: GpuStack::start")
    }

    /// `gone` was unplugged: the stack is replanned without it. Buffers that lived on it
    /// import as `DeviceMissing`, and the next commit falls back to shm.
    pub fn unplugged(
        self,
        gone: &DrmNodeId,
        nodes: &mut dyn RenderNodes,
        choice: &GpuChoice,
    ) -> Result<GpuStack, GpuError> {
        let _ = (self, gone, nodes, choice);
        todo!("P1-D4: GpuStack::unplugged")
    }
}
