//! `CssScene`: the layout seam answered by a CSS document (blitz-dom, no default features)
//! instead of arithmetic. The production layout is `casement_layout::RustScene`; this crate
//! exists to check it (S-L6 parity, lane P3-CSS) and is never linked by the compositor.
//!
//! The spec becomes a flat document of absolutely positioned boxes (`document.rs`), `scene.css`
//! places them with custom properties and CSS math, and `read.rs` reads the boxes back. It
//! lays out what `RustScene` lays out today with these exceptions, each recorded in
//! `FINDINGS.md` ("casement-scene-css: the exclusions") and named in `tests/parity.rs`:
//!
//! - Mission Control's spread (`Arrangement::overview`, `LayoutReport::SpreadOverflow`): an
//!   iterative search, not an expression. `CssScene` returns no slots.
//! - A cascade step that is not the same positive length on both axes (the production knob is
//!   one titlebar height on both).
//! - Coordinates beyond what an f32 holds exactly (about 16 million logical pixels).

mod document;
mod read;

use casement_layout::{Arrangement, LayoutKnobs, SceneLayout, SceneSpec};

/// The CSS implementation of [`SceneLayout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssScene;

impl SceneLayout for CssScene {
    fn arrange(&self, spec: &SceneSpec, knobs: &LayoutKnobs) -> Arrangement {
        let document = document::build(spec, knobs);
        read::arrangement(&document, spec)
    }
}
