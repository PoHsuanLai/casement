//! Drawing one output's elements through smithay's damage tracker.

use crate::compositor::Compositor;
use crate::element::CasementElement;
use casement_core::geometry::PhysicalRect;
use smithay::backend::renderer::damage::OutputDamageTracker;

/// What one render produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// The damaged area, in the output's physical pixels (empty when nothing changed).
    pub damage: Vec<PhysicalRect>,
    /// Whether the frame is finished or waits on a fence.
    pub sync: RenderSync,
}

/// Whether a rendered frame is complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderSync {
    /// The frame is complete.
    Done,
    /// The GPU signals a fence when it is.
    Fence,
}

/// Why rendering failed.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// The renderer refused.
    #[error("renderer: {0}")]
    Renderer(String),
    /// The output has nothing to render into.
    #[error("output has no target")]
    NoTarget,
}

/// Draws `elements` (front first, smithay's order) into `target` with `tracker`, which knows
/// the output's mode, scale and transform. `age` is the target's buffer age (0 when unknown:
/// everything is redrawn). Waits for nothing: [`Rendered::sync`] says whether a fence is
/// pending.
pub fn render_output<R: Compositor>(
    renderer: &mut R,
    target: &mut R::Framebuffer<'_>,
    tracker: &mut OutputDamageTracker,
    elements: &[CasementElement<R>],
    age: usize,
) -> Result<Rendered, RenderError> {
    let _ = (renderer, target, tracker, elements, age);
    todo!("P0-R-scene: render_output")
}
