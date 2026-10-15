//! The DOM a scene becomes: one element per output, Space and window, with the knobs as CSS
//! custom properties, styled by `scene.css`.
//!
//! Every box is a `<div>` directly under `<body>` (a Split View's halves hold their windows), so
//! a box's laid-out position is its global one. What each box is, and which node is which, is
//! kept beside the DOM in [`OutputNodes`] so `read.rs` follows the spec's own order.

mod cascade;
mod desktop;
mod split;
mod style;
mod tree;

use self::style::Props;
use self::tree::Tree;
use blitz_dom::{BaseDocument, DocumentConfig, NodeId};
use casement_layout::{LayoutKnobs, OutputSpec, SceneMode, SceneSpec, SpaceArrangement};
use std::fmt;

/// `scene.css`: the layout rules.
const SCENE_CSS: &str = include_str!("scene.css");

/// The scene as a blitz-dom document, laid out.
pub struct SceneDocument {
    /// The laid-out document.
    pub dom: BaseDocument,
    /// The nodes of each output of the spec, in the spec's order.
    pub outputs: Vec<OutputNodes>,
}

// BaseDocument has no Debug: the nodes say what the document holds.
impl fmt::Debug for SceneDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SceneDocument")
            .field("outputs", &self.outputs)
            .finish_non_exhaustive()
    }
}

/// One output's nodes.
#[derive(Debug)]
pub struct OutputNodes {
    /// Its Spaces' nodes, in the spec's order.
    pub spaces: Vec<SpaceNodes>,
}

/// One Space's nodes.
#[derive(Debug)]
pub struct SpaceNodes {
    /// The box whose position is the Space's origin.
    pub origin: NodeId,
    /// Its windows.
    pub body: SpaceBody,
}

/// What a Space's windows are laid out as.
#[derive(Debug)]
pub enum SpaceBody {
    /// Windows on their own, in the spec's order (a minimised or tiled one has no node).
    Desktop(Vec<WindowNode>),
    /// A Split View; `None` when the pair names a window the Space does not hold, so nothing is
    /// arranged.
    Split(Option<SplitNodes>),
}

/// A window's node: `index` is the window's place in its `SpaceSpec::windows`.
#[derive(Debug, Clone, Copy)]
pub struct WindowNode {
    /// The place in `SpaceSpec::windows`.
    pub index: usize,
    /// The window's box.
    pub node: NodeId,
}

/// A Split View's nodes.
#[derive(Debug)]
pub struct SplitNodes {
    /// The strip between the halves.
    pub strip: NodeId,
    /// The leading window.
    pub leading: WindowNode,
    /// The trailing window.
    pub trailing: WindowNode,
}

/// The document for `spec` under `knobs`, laid out.
pub fn build(spec: &SceneSpec, knobs: &LayoutKnobs) -> SceneDocument {
    let mut dom = BaseDocument::new(DocumentConfig::default());
    dom.add_user_agent_stylesheet(SCENE_CSS);
    let outputs = {
        let mut tree = Tree::open(&mut dom);
        spec.outputs
            .iter()
            .map(|output| output_nodes(&mut tree, output, knobs))
            .collect()
    };
    dom.resolve(0.0);
    SceneDocument { dom, outputs }
}

/// `output`'s boxes: each Space's origin and, under it in the spec, the Space's windows.
fn output_nodes(tree: &mut Tree<'_>, output: &OutputSpec, knobs: &LayoutKnobs) -> OutputNodes {
    let shown = shown_index(output);
    let spaces = output
        .spaces
        .iter()
        .enumerate()
        .map(|(index, space)| {
            let apart = i64::try_from(index).unwrap_or(0) - i64::try_from(shown).unwrap_or(0);
            let body_node = tree.body();
            let origin = tree.div(body_node, "space", origin_props(output, apart));
            let body = match space.kind {
                SpaceArrangement::Split(pair) => {
                    SpaceBody::Split(split::build(tree, output, space, pair))
                }
                SpaceArrangement::Desktop | SpaceArrangement::FullScreen(_) => {
                    SpaceBody::Desktop(desktop::build(tree, output, space, knobs))
                }
            };
            SpaceNodes { origin, body }
        })
        .collect();
    OutputNodes { spaces }
}

fn origin_props(output: &OutputSpec, apart: i64) -> Props {
    Props::new()
        .px("ux", output.usable.origin.x)
        .px("uy", output.usable.origin.y)
        .px("uw", output.usable.size.w)
        .num("apart", apart)
}

/// The row position of the Space the output shows (the first when it names none).
fn shown_index(output: &OutputSpec) -> usize {
    let shown = match &output.mode {
        SceneMode::Desktop { shown } | SceneMode::Overview { shown, .. } => Some(shown),
        SceneMode::ShowDesktop => None,
    };
    shown
        .and_then(|shown| output.spaces.iter().position(|space| &space.space == shown))
        .unwrap_or(0)
}
