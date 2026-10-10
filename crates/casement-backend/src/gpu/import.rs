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

/// How a GPU takes another GPU's buffers (smithay's `can_do_cross_device_imports` and
/// `should_do_cross_device_exports`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CrossSupport {
    /// Directly: smithay's dma shadow copy.
    Direct,
    /// Only through the CPU (`ExportMem`, `ImportMem`).
    CopyOnly,
    /// Not at all.
    Refuse,
}

/// The route for a buffer that lives on `source` under `plan`, when the render GPU takes
/// foreign buffers directly. A `source` the plan does not name is refused: nothing can say
/// which GPU holds the buffer.
///
/// ```
/// use casement_backend::gpu::{ImportRoute, route_for};
/// use casement_core::gpu::{DrmNodeId, RenderPlan};
/// use std::collections::BTreeMap;
///
/// let render = DrmNodeId::new(226, 128);
/// let plan = RenderPlan::new(render, BTreeMap::new());
/// assert_eq!(route_for(&render, &plan), ImportRoute::Local);
/// assert_eq!(route_for(&DrmNodeId::new(226, 129), &plan), ImportRoute::Refused);
/// ```
pub fn route_for(source: &DrmNodeId, plan: &RenderPlan) -> ImportRoute {
    route_with(source, plan, CrossSupport::Direct)
}

/// The route for a buffer on `source`, given how the render GPU takes foreign buffers.
pub fn route_with(source: &DrmNodeId, plan: &RenderPlan, cross: CrossSupport) -> ImportRoute {
    if *source == plan.render {
        return ImportRoute::Local;
    }
    if !plan.scanout.values().any(|node| node == source) {
        return ImportRoute::Refused;
    }
    match cross {
        CrossSupport::Direct => ImportRoute::Direct,
        CrossSupport::CopyOnly => ImportRoute::Copy,
        CrossSupport::Refuse => ImportRoute::Refused,
    }
}

/// The node a client's dma-buf is tagged with: the device of the feedback tranche the client
/// allocated from, which the dmabuf handler passes to `Dmabuf::set_node`. A target the plan
/// does not name (a stale tranche) is tagged with the render GPU, the only GPU the compositor
/// knows to hold it.
///
/// ```
/// use casement_backend::gpu::tag_node;
/// use casement_core::gpu::{DrmNodeId, RenderPlan};
/// use std::collections::BTreeMap;
///
/// let (a, b) = (DrmNodeId::new(226, 128), DrmNodeId::new(226, 129));
/// let plan = RenderPlan::new(a, BTreeMap::new());
/// assert_eq!(tag_node(&plan, &a), a);
/// assert_eq!(tag_node(&plan, &b), a);
/// ```
pub fn tag_node(plan: &RenderPlan, tranche_target: &DrmNodeId) -> DrmNodeId {
    let named =
        *tranche_target == plan.render || plan.scanout.values().any(|node| node == tranche_target);
    if named { *tranche_target } else { plan.render }
}
