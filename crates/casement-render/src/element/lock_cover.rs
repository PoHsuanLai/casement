//! The cover drawn over a locked output that has no live lock surface (03 §1.13: a dead locker
//! never reveals the session).

use super::solid::{Rgba8, fingerprint};
use crate::scene::convert::physical;
use casement_core::geometry::PhysicalRect;
use smithay::backend::renderer::element::{Element, Id, RenderElement};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::backend::renderer::{Frame, Renderer};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Physical, Rectangle, Scale};

/// An opaque cover over a whole output.
#[derive(Debug, Clone)]
pub struct LockCoverElement {
    id: Id,
    area: PhysicalRect,
    colour: Rgba8,
}

impl LockCoverElement {
    /// A cover over `area` (output-local physical pixels) in `colour` (opaque by rule: a
    /// translucent colour is drawn with full alpha).
    pub fn new(id: Id, area: PhysicalRect, colour: Rgba8) -> LockCoverElement {
        LockCoverElement { id, area, colour }
    }

    /// The colour as drawn.
    fn drawn(&self) -> Rgba8 {
        self.colour.opaque()
    }

    /// The cover's own extent, at its origin.
    fn extent(&self) -> Rectangle<i32, Physical> {
        Rectangle::from_size(physical(self.area).size)
    }
}

impl Element for LockCoverElement {
    fn id(&self) -> &Id {
        &self.id
    }

    /// The content is the colour alone (the tracker follows the geometry), so the colour is
    /// the commit: a new colour damages the whole cover, an unchanged one nothing.
    fn current_commit(&self) -> CommitCounter {
        let Rgba8 { r, g, b, a } = self.drawn();
        fingerprint([r, g, b, a])
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        let size = physical(self.area).size;
        Rectangle::from_size((f64::from(size.w), f64::from(size.h)).into())
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        let _ = scale;
        physical(self.area)
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        let _ = scale;
        match commit == Some(self.current_commit()) {
            true => DamageSet::default(),
            false => DamageSet::from_slice(&[self.extent()]),
        }
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        let _ = scale;
        OpaqueRegions::from_slice(&[self.extent()])
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
        let _ = (src, opaque_regions, cache);
        frame.draw_solid(dst, damage, self.drawn().color32f())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cover(colour: Rgba8) -> LockCoverElement {
        let area = PhysicalRect {
            x: 10,
            y: 20,
            w: 300,
            h: 200,
        };
        LockCoverElement::new(Id::new(), area, colour)
    }

    const GREY: Rgba8 = Rgba8 {
        r: 40,
        g: 40,
        b: 40,
        a: 128,
    };

    #[test]
    fn a_cover_is_opaque_over_its_area_whatever_its_colour_says() {
        let cover = cover(GREY);
        let scale = Scale::from(2.0);
        assert_eq!(
            cover.geometry(scale),
            Rectangle::new((10, 20).into(), (300, 200).into())
        );
        let opaque: Vec<_> = cover.opaque_regions(scale).into_iter().collect();
        assert_eq!(opaque, vec![Rectangle::from_size((300, 200).into())]);
        assert_eq!(cover.drawn(), Rgba8 { a: 255, ..GREY });
    }

    #[test]
    fn a_cover_damages_itself_only_when_its_colour_changes() {
        let cover = cover(GREY);
        let scale = Scale::from(1.0);
        let now = cover.current_commit();
        assert_eq!(cover.damage_since(scale, Some(now)).len(), 0);
        assert_eq!(cover.damage_since(scale, None).len(), 1);
        let other = LockCoverElement::new(cover.id.clone(), cover.area, Rgba8 { r: 0, ..GREY });
        assert_ne!(other.current_commit(), now);
    }
}
