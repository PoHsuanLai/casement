//! The cover drawn over a locked output that has no live lock surface (03 §1.13: a dead locker
//! never reveals the session).

use super::solid::Rgba8;
use casement_core::geometry::PhysicalRect;
use smithay::backend::renderer::Renderer;
use smithay::backend::renderer::element::{Element, Id, RenderElement};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Physical, Rectangle, Scale};

/// An opaque cover over a whole output.
#[derive(Debug, Clone)]
pub struct LockCoverElement {
    #[expect(dead_code, reason = "P0-R-scene: the element's id")]
    id: Id,
    #[expect(dead_code, reason = "P0-R-scene: the covered area")]
    area: PhysicalRect,
    #[expect(dead_code, reason = "P0-R-scene: the cover's colour")]
    colour: Rgba8,
}

impl LockCoverElement {
    /// A cover over `area` (output-local physical pixels) in `colour` (opaque by rule).
    pub fn new(id: Id, area: PhysicalRect, colour: Rgba8) -> LockCoverElement {
        LockCoverElement { id, area, colour }
    }
}

impl Element for LockCoverElement {
    fn id(&self) -> &Id {
        todo!("P0-R-scene: LockCoverElement::id")
    }

    fn current_commit(&self) -> CommitCounter {
        todo!("P0-R-scene: LockCoverElement::current_commit")
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        todo!("P0-R-scene: LockCoverElement::src")
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        let _ = scale;
        todo!("P0-R-scene: LockCoverElement::geometry")
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        let _ = (scale, commit);
        todo!("P0-R-scene: LockCoverElement::damage_since")
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        let _ = scale;
        todo!("P0-R-scene: LockCoverElement::opaque_regions")
    }
}

impl<R: Renderer> RenderElement<R> for LockCoverElement {
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
        todo!("P0-R-scene: LockCoverElement::draw")
    }
}
