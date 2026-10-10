//! Which GPU renders and which scans each output out.

use super::{BootVga, ByPathId, DrmNodeId, GpuChoice, GpuFacts};
use crate::ids::OutputName;
use std::collections::BTreeMap;

/// The render plan: one GPU renders every frame, each output is scanned out by the GPU that
/// drives it (a frame rendered elsewhere is copied across, SP4).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RenderPlan {
    /// The GPU that renders.
    pub render: DrmNodeId,
    /// The GPU that scans each connector out. (The plan sketch keys this by `OutputKey`;
    /// detection knows connectors by name, and a display's key needs its EDID: FINDINGS.)
    pub scanout: BTreeMap<OutputName, DrmNodeId>,
}

impl RenderPlan {
    /// A plan.
    pub fn new(render: DrmNodeId, scanout: BTreeMap<OutputName, DrmNodeId>) -> RenderPlan {
        RenderPlan { render, scanout }
    }
}

/// Why no plan could be made.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PlanRefusal {
    /// Detection found no GPU.
    #[error("no GPU")]
    NoGpu,
    /// The setting names a GPU that is not there.
    #[error("the GPU {} is not present", .0.as_str())]
    UnknownGpu(ByPathId),
}

/// The plan for `facts` under `choice`. The GPU driving the outputs wins; among several, the
/// firmware's display GPU; the setting overrides both.
pub fn plan_for(facts: &[GpuFacts], choice: &GpuChoice) -> Result<RenderPlan, PlanRefusal> {
    let render = match choice {
        GpuChoice::Auto => automatic(facts).ok_or(PlanRefusal::NoGpu)?,
        GpuChoice::ByPath(id) => facts
            .iter()
            .find(|gpu| &gpu.by_path == id)
            .map(|gpu| gpu.node)
            .ok_or_else(|| PlanRefusal::UnknownGpu(id.clone()))?,
    };
    let scanout = facts
        .iter()
        .flat_map(|gpu| gpu.drives.iter().map(|name| (name.clone(), gpu.node)))
        .collect();
    Ok(RenderPlan::new(render, scanout))
}

/// The GPU that renders when the setting does not say: of the GPUs driving a monitor the
/// firmware's display GPU, else the lowest node; with none driving one, the same rule over all.
fn automatic(facts: &[GpuFacts]) -> Option<DrmNodeId> {
    let driving: Vec<&GpuFacts> = facts.iter().filter(|gpu| !gpu.drives.is_empty()).collect();
    let pool: Vec<&GpuFacts> = if driving.is_empty() {
        facts.iter().collect()
    } else {
        driving
    };
    pool.into_iter()
        .min_by_key(|gpu| (gpu.boot_vga != BootVga::Primary, gpu.node))
        .map(|gpu| gpu.node)
}
