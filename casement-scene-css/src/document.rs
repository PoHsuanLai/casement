//! The DOM a scene becomes: one element per output, Space and window, with the knobs as CSS
//! custom properties, styled by `scene.css`.

use casement_layout::{LayoutKnobs, SceneSpec};

/// The scene as a blitz-dom document, laid out.
#[derive(Debug)]
pub struct SceneDocument {
    /// The laid-out document.
    pub dom: blitz_dom::BaseDocument,
}

/// The document for `spec` under `knobs`, laid out.
pub fn build(spec: &SceneSpec, knobs: &LayoutKnobs) -> SceneDocument {
    let _ = (spec, knobs);
    todo!("P3-CSS")
}
