//! `CssScene`: the layout seam answered by a CSS document (blitz-dom, no default features)
//! instead of arithmetic. The production layout is `casement_layout::RustScene`; this crate
//! exists to check it (S-L6 parity, lane P3-CSS) and is never linked by the compositor.

mod document;
mod read;

use casement_layout::{Arrangement, LayoutKnobs, SceneLayout, SceneSpec};

/// The CSS implementation of [`SceneLayout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssScene;

impl SceneLayout for CssScene {
    fn arrange(&self, spec: &SceneSpec, knobs: &LayoutKnobs) -> Arrangement {
        let _ = (spec, knobs);
        todo!("P3-CSS: document::build, lay out with scene.css, read::arrangement")
    }
}
