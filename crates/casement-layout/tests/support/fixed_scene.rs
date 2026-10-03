//! `FixedScene`: a layout that answers one fixed arrangement, for tests of what consumes
//! `SceneLayout` (motion, the composition root) without RustScene's rules.

use casement_layout::{Arrangement, LayoutKnobs, SceneLayout, SceneSpec};

/// Always arranges as `.0`, whatever the spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedScene(pub Arrangement);

impl SceneLayout for FixedScene {
    fn arrange(&self, _spec: &SceneSpec, _knobs: &LayoutKnobs) -> Arrangement {
        self.0.clone()
    }
}
