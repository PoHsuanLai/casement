//! Which GPU renders and which scans each output out.

use super::{ByPathId, DrmNodeId, GpuChoice, GpuFacts};
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
    let _ = (facts, choice);
    todo!("P1-D4: plan_for")
}
