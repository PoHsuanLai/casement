//! The headless renderers: pixman (tier P goldens, byte-exact), llvmpipe GLES on the EGL
//! surfaceless platform (tier L, effects), and smithay's dummy renderer (tests that draw
//! nothing, `testing` only).

use crate::backend::BackendError;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::pixman::PixmanRenderer;

/// Which software renderer the headless backend uses (`--renderer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoftRenderer {
    /// smithay's dummy renderer: nothing is drawn (`testing` only).
    Dummy,
    /// pixman: exact, no effects.
    Pixman,
    /// GLES on Mesa's llvmpipe: effects run.
    Llvmpipe,
}

/// A started headless renderer.
#[derive(Debug)]
pub enum HeadlessRenderer {
    /// smithay's dummy renderer.
    #[cfg(feature = "testing")]
    Dummy(Box<smithay::backend::renderer::test::DummyRenderer>),
    /// pixman.
    Pixman(Box<PixmanRenderer>),
    /// GLES on llvmpipe (`casement_render::surfaceless_gles`, the one unsafe module).
    Gles(Box<GlesRenderer>),
}

impl HeadlessRenderer {
    /// Starts `choice` (the dummy renderer only under `testing`).
    pub fn start(choice: SoftRenderer) -> Result<HeadlessRenderer, BackendError> {
        let _ = choice;
        todo!("P0-B-headless: HeadlessRenderer::start")
    }
}
