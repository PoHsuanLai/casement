//! Where the dma-buf format tables come from (P1-Z; filled by P1-D4): each render node's
//! renderable and importable formats, read once per node. The tranches built from them are
//! `casement_core::gpu::feedback_for`'s.

use super::{GpuError, RenderNodes};
use casement_core::gpu::{DrmNodeId, FormatEntry, FormatTable, RenderPlan};
use std::collections::{BTreeMap, BTreeSet};

/// A node's formats. Real: the node's EGL display; fake: `casement_testkit::gpu::FormatSet`.
pub trait FormatSource: std::fmt::Debug {
    /// The formats `node` renders to and imports.
    fn formats_of(&mut self, node: &DrmNodeId) -> Result<Vec<FormatEntry>, GpuError>;
}

/// The table for every node the plan names (the render GPU and each scanout GPU), each read
/// once. A node detection does not list is `GpuError::UnknownNode`.
pub fn format_table(
    source: &mut dyn FormatSource,
    nodes: &dyn RenderNodes,
    plan: &RenderPlan,
) -> Result<FormatTable, GpuError> {
    let known: BTreeSet<DrmNodeId> = nodes.nodes().into_iter().map(|gpu| gpu.node).collect();
    let wanted: BTreeSet<DrmNodeId> = std::iter::once(plan.render)
        .chain(plan.scanout.values().copied())
        .collect();
    let mut per_node = BTreeMap::new();
    for node in wanted {
        if !known.contains(&node) {
            return Err(GpuError::UnknownNode(node));
        }
        per_node.insert(node, source.formats_of(&node)?);
    }
    Ok(FormatTable::new(per_node))
}
