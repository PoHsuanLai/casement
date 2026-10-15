//! A desktop Space's windows: one box per window that has a target, its placement as the class
//! and its spec as custom properties.

use super::WindowNode;
use super::style::Props;
use super::tree::Tree;
use casement_layout::{LayoutKnobs, OutputSpec, SpaceSpec, WindowPlacement, WindowSpec};

/// The boxes of `space`'s windows, in the spec's order; a minimised or tiled window has none.
pub(super) fn build(
    tree: &mut Tree<'_>,
    output: &OutputSpec,
    space: &SpaceSpec,
    knobs: &LayoutKnobs,
) -> Vec<WindowNode> {
    let body = tree.body();
    let mut new_so_far = 0_i64;
    let mut nodes = Vec::new();
    for (index, window) in space.windows.iter().enumerate() {
        let nth = new_so_far;
        let Some((class, props)) = describe(output, window, knobs, nth) else {
            continue;
        };
        if matches!(window.placement, WindowPlacement::New(_)) {
            new_so_far += 1;
        }
        let node = tree.div(body, class, props);
        nodes.push(WindowNode { index, node });
    }
    nodes
}

/// The class and the custom properties of `window`; `None` for a window with no target.
fn describe(
    output: &OutputSpec,
    window: &WindowSpec,
    knobs: &LayoutKnobs,
    nth: i64,
) -> Option<(&'static str, Props)> {
    let common = Props::new()
        .rect("u", output.usable)
        .px("reserve", window.reserve_top)
        .limits(window.limits);
    let (class, props) = match window.placement {
        WindowPlacement::Floating(rect) => ("win floating", common.rect("", rect)),
        WindowPlacement::Placed(rect) => ("win placed", common.rect("", rect)),
        WindowPlacement::New(asked) => (
            "win new",
            common
                .size(asked)
                .num("nth", nth)
                .px("step", cascade_step(knobs)),
        ),
        WindowPlacement::Filled => ("win filled", common),
        WindowPlacement::FullScreen => ("win fullscreen", common.rect("full-", output.full)),
        WindowPlacement::Minimised | WindowPlacement::Tiled(_) => return None,
    };
    Some((class, props))
}

/// The cascade step: one length for both axes (the production knob is one titlebar height).
/// Unequal, zero or negative steps are outside what `scene.css` models (FINDINGS.md).
fn cascade_step(knobs: &LayoutKnobs) -> i32 {
    knobs.cascade_step.x.max(1)
}
