//! The dma-buf feedback a client is offered: tranches of formats per device, most preferred
//! first (render formats, then scanout, importable-from-elsewhere last).

use super::{DrmNodeId, RenderPlan};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A DRM fourcc code (`AR24` is ARGB8888), as the kernel spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Fourcc(pub u32);

/// A DRM format modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Modifier(pub u64);

/// One format and the modifier it is usable with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FormatEntry {
    /// The pixel format.
    pub format: Fourcc,
    /// The layout.
    pub modifier: Modifier,
}

/// The formats each node can render to or import.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct FormatTable {
    /// Each node's formats.
    pub per_node: BTreeMap<DrmNodeId, Vec<FormatEntry>>,
    /// The formats the primary planes of the outputs each GPU scans out accept (what a client
    /// may allocate so that its buffer goes to the screen without a copy). Empty for a node
    /// with no lit output, and for a table made without KMS.
    pub scanout: BTreeMap<DrmNodeId, Vec<FormatEntry>>,
}

impl FormatTable {
    /// A table of what each node renders to and imports, with no scanout formats.
    pub fn new(per_node: BTreeMap<DrmNodeId, Vec<FormatEntry>>) -> FormatTable {
        FormatTable {
            per_node,
            scanout: BTreeMap::new(),
        }
    }

    /// The same table with the formats the primary planes of each GPU's outputs accept.
    #[must_use]
    pub fn with_scanout(self, scanout: BTreeMap<DrmNodeId, Vec<FormatEntry>>) -> FormatTable {
        FormatTable { scanout, ..self }
    }
}

/// What a tranche is for (`zwp_linux_dmabuf_feedback_v1.tranche_flags`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TrancheUse {
    /// Formats the render GPU draws into directly.
    Render,
    /// Formats a plane can scan out.
    Scanout,
    /// Formats the render GPU imports from another GPU, after a copy.
    Import,
}

/// One tranche: a target device, what it is for and its formats.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Tranche {
    /// The device whose formats these are.
    pub target: DrmNodeId,
    /// What the tranche is for.
    pub purpose: TrancheUse,
    /// Its formats.
    pub formats: Vec<FormatEntry>,
}

/// The whole feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FeedbackPlan {
    /// The main device (the render GPU).
    pub main_device: DrmNodeId,
    /// The tranches, most preferred first.
    pub tranches: Vec<Tranche>,
}

/// The feedback for `plan`, from each node's `formats`: first, when the render GPU also scans an
/// output out, the formats its primary planes accept that it also draws into (a client that
/// allocates there is put on the screen without a copy: the `Scanout` tranche); then what the
/// render GPU draws into (a client that allocates there is composited without a copy); then,
/// for every other GPU that scans an output out, the formats the render GPU would import from
/// it after a copy. Each format-and-modifier is offered once, at its first tranche. Empty
/// tranches are left out.
///
/// Only the render GPU's own planes make a `Scanout` tranche: a plane of another GPU cannot
/// show a buffer the render GPU drew without the copy the `Import` tranche stands for.
///
/// ```
/// use casement_core::gpu::{DrmNodeId, FormatTable, RenderPlan, feedback_for};
/// use std::collections::{BTreeMap, BTreeSet};
///
/// let render = DrmNodeId::new(226, 128);
/// let plan = RenderPlan::new(render, BTreeMap::new());
/// let feedback = feedback_for(&plan, &FormatTable::default());
/// assert_eq!(feedback.main_device, render);
/// assert!(feedback.tranches.is_empty());
/// ```
pub fn feedback_for(plan: &RenderPlan, formats: &FormatTable) -> FeedbackPlan {
    let mut offered: BTreeSet<FormatEntry> = BTreeSet::new();
    let mut tranches = Vec::new();
    let mut push = |target: DrmNodeId, purpose: TrancheUse, candidates: Vec<FormatEntry>| {
        let fresh: Vec<FormatEntry> = candidates
            .into_iter()
            .filter(|entry| offered.insert(*entry))
            .collect();
        if !fresh.is_empty() {
            tranches.push(Tranche {
                target,
                purpose,
                formats: fresh,
            });
        }
    };
    let own = |node: &DrmNodeId| formats.per_node.get(node).cloned().unwrap_or_default();
    let direct: Vec<FormatEntry> = own(&plan.render)
        .into_iter()
        .filter(|entry| {
            formats
                .scanout
                .get(&plan.render)
                .is_some_and(|planes| planes.contains(entry))
        })
        .collect();
    push(plan.render, TrancheUse::Scanout, direct);
    push(plan.render, TrancheUse::Render, own(&plan.render));
    let others: BTreeSet<DrmNodeId> = plan
        .scanout
        .values()
        .copied()
        .filter(|node| *node != plan.render)
        .collect();
    for node in others {
        push(node, TrancheUse::Import, own(&node));
    }
    FeedbackPlan {
        main_device: plan.render,
        tranches,
    }
}
