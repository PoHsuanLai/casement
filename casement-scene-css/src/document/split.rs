//! A Split View Space: a flex row over the whole output (the leading half, the divider strip,
//! the trailing half), each window in its half.

use super::style::Props;
use super::tree::Tree;
use super::{SplitNodes, WindowNode};
use casement_core::WindowId;
use casement_core::space::SplitPair;
use casement_layout::{OutputSpec, SpaceSpec, WindowSpec};

/// The row's nodes; `None` when the pair names a window the Space does not hold (RustScene
/// arranges nothing then, and so does the document).
pub(super) fn build(
    tree: &mut Tree<'_>,
    output: &OutputSpec,
    space: &SpaceSpec,
    pair: SplitPair,
) -> Option<SplitNodes> {
    let (lead_at, leading) = find(space, pair.leading)?;
    let (trail_at, trailing) = find(space, pair.trailing)?;
    let row = Props::new()
        .rect("a", output.full)
        .num("share", pair.divider.0)
        .px("min-lead", leading.limits.min.w)
        .px("min-trail", trailing.limits.min.w);
    let body = tree.body();
    let row = tree.div(body, "split", row);
    let lead = tree.div(row, "part lead", Props::new());
    let strip = tree.div(row, "part strip", Props::new());
    let trail = tree.div(row, "part trail", Props::new());
    let leading_node = tree.div(lead, "swin", held(leading));
    let trailing_node = tree.div(trail, "swin", held(trailing));
    Some(SplitNodes {
        strip,
        leading: WindowNode {
            index: lead_at,
            node: leading_node,
        },
        trailing: WindowNode {
            index: trail_at,
            node: trailing_node,
        },
    })
}

fn find(space: &SpaceSpec, id: WindowId) -> Option<(usize, &WindowSpec)> {
    space
        .windows
        .iter()
        .enumerate()
        .find(|(_, spec)| spec.window == id)
}

/// A window's maximum as `--max-w`/`--max-h`; 0 means none (`within_max` reads 0 as "the
/// half's own extent", which is also what no maximum means).
fn held(window: &WindowSpec) -> Props {
    let (w, h) = match window.limits.max {
        None => (0, 0),
        Some(max) => (max.w, max.h),
    };
    Props::new().px("max-w", w).px("max-h", h)
}
