//! Client buffers across GPUs (P1-Z; filled by P1-D4): which path a buffer takes to reach the
//! render GPU, and the node it is tagged with (A5: an untagged dma-buf silently skips the copy).

use casement_core::gpu::{DrmNodeId, RenderPlan};

/// How a client buffer reaches the render GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImportRoute {
    /// The buffer is already on the render GPU.
    Local,
    /// The render GPU imports it directly (a dma shadow copy).
    Direct,
    /// It is copied through the CPU.
    Copy,
    /// It cannot be imported: the client is told to use shm.
    Refused,
}

/// The route for a buffer that lives on `source` under `plan`.
pub fn route_for(source: &DrmNodeId, plan: &RenderPlan) -> ImportRoute {
    let _ = (source, plan);
    todo!("P1-D4: route_for")
}

/// The node a client's dma-buf is tagged with: the device of the feedback tranche the client
/// allocated from, which the dmabuf handler passes to `Dmabuf::set_node`.
pub fn tag_node(plan: &RenderPlan, tranche_target: &DrmNodeId) -> DrmNodeId {
    let _ = (plan, tranche_target);
    todo!("P1-D4: tag_node")
}
