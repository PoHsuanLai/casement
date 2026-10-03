//! Taffy trees: made without Taffy's own rounding (the bridge rounds edges itself, so a Split's
//! `1147.2 / 764.8` lands as `1147 / 765`), solved inside a logical area.

use super::round;
use casement_core::geometry::LogicalSize;
use taffy::{AvailableSpace, NodeId, Size, TaffyError, TaffyTree};

/// An empty tree with Taffy's rounding off.
pub(crate) fn new_tree() -> TaffyTree<()> {
    let mut tree = TaffyTree::new();
    tree.disable_rounding();
    tree
}

/// Lays `tree` out from `root` within `within`.
pub(crate) fn solve(
    tree: &mut TaffyTree<()>,
    root: NodeId,
    within: LogicalSize,
) -> Result<(), TaffyError> {
    let space = Size {
        width: AvailableSpace::Definite(round::to_float(within.w)),
        height: AvailableSpace::Definite(round::to_float(within.h)),
    };
    tree.compute_layout(root, space)
}
