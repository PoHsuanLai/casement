//! A scene with nothing in it, for suites whose fake backend never draws.

use casement_backend::headless::FrameElements;
use casement_render::{CasementElement, Compositor};

/// A scene with no elements.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyScene;

impl FrameElements for EmptyScene {
    fn elements<R: Compositor>(&self, _: &mut R) -> Vec<CasementElement<R>> {
        Vec::new()
    }
}
