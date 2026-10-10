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
}

impl FormatTable {
    /// A table.
    pub fn new(per_node: BTreeMap<DrmNodeId, Vec<FormatEntry>>) -> FormatTable {
        FormatTable { per_node }
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

/// The feedback for `plan`, from each node's `formats`: first what the render GPU draws into
/// (a client that allocates there is composited without a copy), then, for every other GPU
/// that scans an output out, the formats the render GPU would import from it after a copy,
/// each format-and-modifier once, at its first tranche. Empty tranches are left out.
///
/// There is no `Scanout` tranche yet: it needs the KMS planes' formats, which the format table
/// does not carry.
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
    let mut push = |target: DrmNodeId, purpose: TrancheUse| {
        let fresh: Vec<FormatEntry> = formats
            .per_node
            .get(&target)
            .into_iter()
            .flatten()
            .filter(|entry| offered.insert(**entry))
            .copied()
            .collect();
        if !fresh.is_empty() {
            tranches.push(Tranche {
                target,
                purpose,
                formats: fresh,
            });
        }
    };
    push(plan.render, TrancheUse::Render);
    let others: BTreeSet<DrmNodeId> = plan
        .scanout
        .values()
        .copied()
        .filter(|node| *node != plan.render)
        .collect();
    for node in others {
        push(node, TrancheUse::Import);
    }
    FeedbackPlan {
        main_device: plan.render,
        tranches,
    }
}
