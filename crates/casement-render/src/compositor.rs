//! The renderer trait casement composites with.

use crate::element::{BlurV0Element, RoundedClipElement};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::backend::renderer::{ImportAll, ImportMem, Renderer, RendererSuper};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer, Physical, Rectangle};

/// The renderers casement composites with: smithay's traits plus what our effect passes need.
///
/// `ExportMem` (reading a frame back) is not a supertrait: smithay's `DummyRenderer` has none,
/// so capture and goldens ask for it where they read pixels (a departure from the plan's
/// sketch, FINDINGS). Textures are `Clone + Send + 'static`, which smithay's surface and
/// memory elements need (true of every renderer casement uses).
///
/// The effect hooks have default bodies, the solid stand-ins of [`EffectSupport::Plain`]; a
/// renderer with shaders overrides them. [`crate::render_output`] routes effect elements
/// through them (a `render_elements!` enum cannot name a renderer per variant).
pub trait Compositor:
    RendererSuper<TextureId: Clone + Send + 'static> + Renderer + ImportAll + ImportMem + Sized
{
    /// Whether this renderer runs effect shaders or draws their solid stand-ins.
    fn effects(&mut self) -> EffectSupport;

    /// Blur v0, before its draw: copy what lies under `dst` into `cache` (smithay calls it
    /// whenever that changed). The stand-in needs nothing.
    fn capture_blur_v0(
        frame: &mut Self::Frame<'_, '_>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), Self::Error> {
        let _ = (frame, dst, cache);
        Ok(())
    }

    /// Blur v0: draw `blur` at `dst` where `damage` (relative to `dst`) asks. The default is
    /// the stand-in colour over the region.
    fn draw_blur_v0(
        frame: &mut Self::Frame<'_, '_>,
        blur: &BlurV0Element,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), Self::Error> {
        let _ = cache;
        blur.draw_stand_in(frame, dst, damage)
    }

    /// A framed window's surface clipped at its bottom corners: draw `clip` at `dst` where
    /// `damage` (relative to `dst`) asks. The default clips by pieces weighted by coverage
    /// (pixman, the dummy renderer, a multi-GPU copy; FINDINGS, `wave-2026-10-15/corners`); a
    /// renderer with shaders masks the corners in the fragment shader.
    fn draw_rounded_clip(
        frame: &mut Self::Frame<'_, '_>,
        clip: &RoundedClipElement<Self>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), Self::Error> {
        let _ = cache;
        clip.draw_banded(frame, src, dst, damage, opaque_regions)
    }
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

    fn capture_blur_v0(
        frame: &mut Self::Frame<'_, '_>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), Self::Error> {
        BlurV0Element::capture_gles(frame, dst, cache)
    }

    fn draw_blur_v0(
        frame: &mut Self::Frame<'_, '_>,
        blur: &BlurV0Element,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), Self::Error> {
        blur.draw_gles(frame, dst, damage, cache)
    }

    fn draw_rounded_clip(
        frame: &mut Self::Frame<'_, '_>,
        clip: &RoundedClipElement<Self>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), Self::Error> {
        clip.draw_gles(frame, src, dst, damage, opaque_regions, cache)
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
