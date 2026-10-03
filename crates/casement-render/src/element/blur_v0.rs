//! The P0 behind-surface blur (ext-background-effect-v1): a single downsample-and-blur of what
//! lies under a surface's blur region. On renderers without effect shaders
//! ([`crate::EffectSupport::Plain`]) it draws its solid stand-in. P3 replaces it with the dual
//! Kawase pass (`pass/blur_kawase.rs`).

use super::solid::Rgba8;
use casement_core::geometry::PhysicalRect;
use smithay::backend::renderer::Renderer;
use smithay::backend::renderer::element::{Element, Id, RenderElement};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Physical, Rectangle, Scale};

/// A blur of the framebuffer under `region`, tinted with the stand-in colour when shaders are
/// off.
#[derive(Debug, Clone)]
pub struct BlurV0Element {
    #[expect(dead_code, reason = "P0-R-scene: the element's id")]
    id: Id,
    #[expect(dead_code, reason = "P0-R-scene: the blurred region")]
    region: Vec<PhysicalRect>,
    #[expect(dead_code, reason = "P0-R-scene: the stand-in colour")]
    stand_in: Rgba8,
}

impl BlurV0Element {
    /// A blur under `region` (output-local physical pixels), drawn as `stand_in` without
    /// shaders.
    pub fn new(id: Id, region: Vec<PhysicalRect>, stand_in: Rgba8) -> BlurV0Element {
        BlurV0Element {
            id,
            region,
            stand_in,
        }
    }
}

impl Element for BlurV0Element {
    fn id(&self) -> &Id {
        todo!("P0-R-scene: BlurV0Element::id")
    }

    fn current_commit(&self) -> CommitCounter {
        todo!("P0-R-scene: BlurV0Element::current_commit")
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        todo!("P0-R-scene: BlurV0Element::src")
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        let _ = scale;
        todo!("P0-R-scene: BlurV0Element::geometry")
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        let _ = (scale, commit);
        todo!("P0-R-scene: BlurV0Element::damage_since")
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        let _ = scale;
        todo!("P0-R-scene: BlurV0Element::opaque_regions")
    }

    fn is_framebuffer_effect(&self) -> bool {
        todo!("P0-R-scene: BlurV0Element::is_framebuffer_effect")
    }
}

impl<R: Renderer> RenderElement<R> for BlurV0Element {
    fn draw(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), R::Error> {
        let _ = (frame, src, dst, damage, opaque_regions, cache);
        todo!("P0-R-scene: BlurV0Element::draw")
    }

    fn capture_framebuffer(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), R::Error> {
        let _ = (frame, src, dst, cache);
        todo!("P0-R-scene: BlurV0Element::capture_framebuffer")
    }
}
