//! The multi-GPU stack (P1-Z; filled by P1-D4): the render plan, and the GPUs behind it.
//!
//! One GPU renders, each output is scanned out by the GPU that drives it, and a frame rendered
//! elsewhere is copied across (SP4). The copy machinery is smithay's `GpuManager`; the soft
//! topology tests run it over `SoftGraphicsApi` in the testkit, and the tty frame loop (P1-D3)
//! builds the real one from the handles held here.

use super::{GpuError, GpuHandle, RenderNodes};
use casement_core::gpu::{DrmNodeId, GpuChoice, GpuFacts, RenderPlan, plan_for};
use std::collections::BTreeSet;

/// The plan and the GPUs it names.
#[derive(Debug)]
#[non_exhaustive]
pub struct GpuStack {
    /// Who renders and who scans out.
    pub plan: RenderPlan,
    /// An open handle on the render GPU and each scanout GPU, the render GPU first.
    pub handles: Vec<GpuHandle>,
}

impl GpuStack {
    /// Detects the GPUs on `nodes`, makes the plan under `choice` and opens what it names.
    ///
    /// ```
    /// use casement_backend::gpu::{GpuStack, SysRenderNodes, SysRoot};
    /// use casement_core::gpu::GpuChoice;
    ///
    /// let mut nodes = SysRenderNodes::new(SysRoot::under("/nonexistent-sys-root".as_ref()));
    /// assert!(GpuStack::start(&mut nodes, &GpuChoice::Auto).is_err(), "no GPU, no stack");
    /// ```
    pub fn start(nodes: &mut dyn RenderNodes, choice: &GpuChoice) -> Result<GpuStack, GpuError> {
        GpuStack::from_facts(&nodes.nodes(), nodes, choice)
    }

    /// `gone` was unplugged: the stack is replanned without it. Buffers that lived on it
    /// import as `DeviceMissing`, and the next commit falls back to shm.
    pub fn unplugged(
        self,
        gone: &DrmNodeId,
        nodes: &mut dyn RenderNodes,
        choice: &GpuChoice,
    ) -> Result<GpuStack, GpuError> {
        let remaining: Vec<GpuFacts> = nodes
            .nodes()
            .into_iter()
            .filter(|gpu| gpu.node != *gone)
            .collect();
        drop(self);
        GpuStack::from_facts(&remaining, nodes, choice)
    }

    fn from_facts(
        facts: &[GpuFacts],
        nodes: &mut dyn RenderNodes,
        choice: &GpuChoice,
    ) -> Result<GpuStack, GpuError> {
        let plan = plan_for(facts, choice)?;
        let wanted: BTreeSet<DrmNodeId> = plan.scanout.values().copied().collect();
        let others = wanted.into_iter().filter(|node| *node != plan.render);
        let mut handles = Vec::new();
        for node in std::iter::once(plan.render).chain(others) {
            handles.push(nodes.open(&node)?);
        }
        Ok(GpuStack { plan, handles })
    }
}
