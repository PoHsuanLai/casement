//! Where the dma-buf format tables come from (P1-Z; filled by P1-D4): each render node's
//! renderable and importable formats, read once per node. The tranches built from them are
//! `casement_core::gpu::feedback_for`'s.

use super::{GpuError, RenderNodes};
use casement_core::gpu::{DrmNodeId, FormatEntry, FormatTable, RenderPlan};

/// A node's formats. Real: the node's EGL display; fake: `casement_testkit::gpu::FormatSet`.
pub trait FormatSource: std::fmt::Debug {
    /// The formats `node` renders to and imports.
    fn formats_of(&mut self, node: &DrmNodeId) -> Result<Vec<FormatEntry>, GpuError>;
}

/// The table for every node the plan names.
pub fn format_table(
    source: &mut dyn FormatSource,
    nodes: &dyn RenderNodes,
    plan: &RenderPlan,
) -> Result<FormatTable, GpuError> {
    let _ = (source, nodes, plan);
    todo!("P1-D4: format_table")
}
