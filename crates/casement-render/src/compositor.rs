//! The renderer trait casement composites with.

use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::backend::renderer::{ImportAll, ImportMem, Renderer};

/// The renderers casement composites with: smithay's traits plus what our effect passes need.
///
/// `ExportMem` (reading a frame back) is not a supertrait: smithay's `DummyRenderer` has none,
/// so capture and goldens ask for it where they read pixels (a departure from the plan's
/// sketch, FINDINGS).
pub trait Compositor: Renderer + ImportAll + ImportMem + Sized {
    /// Whether this renderer runs effect shaders or draws their solid stand-ins.
    fn effects(&mut self) -> EffectSupport;
}

/// Whether effect passes (blur, shadow, rounded clip, dim, genie, glow) run their shaders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectSupport {
    /// GLES: the shaders run.
    Shaders,
    /// pixman and the dummy renderer: each effect draws its solid stand-in.
    Plain,
}

impl Compositor for GlesRenderer {
    fn effects(&mut self) -> EffectSupport {
        EffectSupport::Shaders
    }
}

impl Compositor for PixmanRenderer {
    fn effects(&mut self) -> EffectSupport {
        EffectSupport::Plain
    }
}

#[cfg(feature = "testing")]
impl Compositor for smithay::backend::renderer::test::DummyRenderer {
    fn effects(&mut self) -> EffectSupport {
        EffectSupport::Plain
    }
}
