//! The dma-buf feedback the tty backend offers (P1-T): the render plan, each node's formats and
//! the formats the KMS planes scan out.

use super::TtyBackend;
use super::device::TtyDevices;
use super::kms::{ConnectorStatus, PlaneKind};
use crate::gpu::{GpuError, format_table};
use casement_core::gpu::{DrmNodeId, FeedbackPlan, FormatEntry, RenderPlan, feedback_for};
use std::collections::{BTreeMap, BTreeSet};

/// The formats the primary planes in front of each GPU's lit-able outputs accept, by the GPU
/// that scans the output out (`plan.scanout`). A connector the plan does not place, and a
/// disconnected one, add nothing.
///
/// ```
/// use casement_backend::tty::{TtyDevices, scanout_formats};
/// use casement_core::gpu::{DrmNodeId, RenderPlan};
/// use std::collections::BTreeMap;
///
/// let plan = RenderPlan::new(DrmNodeId::new(226, 128), BTreeMap::new());
/// assert!(scanout_formats(&plan, &TtyDevices::empty()).is_empty());
/// ```
pub fn scanout_formats(
    plan: &RenderPlan,
    devices: &TtyDevices,
) -> BTreeMap<DrmNodeId, Vec<FormatEntry>> {
    let mut by_node: BTreeMap<DrmNodeId, BTreeSet<FormatEntry>> = BTreeMap::new();
    for device in devices.devices.values() {
        let resources = device.kms.resources();
        let attached = resources
            .connectors
            .iter()
            .filter(|connector| connector.status == ConnectorStatus::Connected);
        for connector in attached {
            let Some(node) = plan.scanout.get(&connector.name) else {
                continue;
            };
            let formats = resources
                .planes
                .iter()
                .filter(|plane| plane.kind == PlaneKind::Primary)
                .filter(|plane| {
                    plane
                        .possible_crtcs
                        .iter()
                        .any(|crtc| connector.possible_crtcs.contains(crtc))
                })
                .flat_map(|plane| plane.formats.iter().copied());
            by_node.entry(*node).or_default().extend(formats);
        }
    }
    by_node
        .into_iter()
        .map(|(node, formats)| (node, formats.into_iter().collect()))
        .collect()
}

impl TtyBackend {
    /// The dma-buf feedback to offer clients: the render plan's tranches over each node's
    /// formats, with the render GPU's scanout tranche from the planes of the devices open now.
    /// None while there is no plan (no GPU was found) or no format source was injected: the
    /// caller then offers what it would offer without a seat (the soft offer, or none).
    pub fn dmabuf_feedback(&mut self) -> Result<Option<FeedbackPlan>, GpuError> {
        let (Some(plan), Some(source)) = (&self.plan, self.seams.formats.as_mut()) else {
            return Ok(None);
        };
        let table = format_table(source.as_mut(), self.seams.nodes.as_ref(), plan)?
            .with_scanout(scanout_formats(plan, &self.devices));
        Ok(Some(feedback_for(plan, &table)))
    }
}
