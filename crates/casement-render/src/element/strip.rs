//! One horizontal strip of an element, drawn somewhere else at another size: the piece the
//! genie is made of. The element is rescaled about the strip's origin, cropped to the strip's
//! band and moved to where the band is drawn, all by smithay's own wrappers, so damage, opaque
//! regions and the source rectangle follow from them.

use smithay::backend::renderer::Renderer;
use smithay::backend::renderer::element::utils::{
    CropRenderElement, Relocate, RelocateRenderElement, RescaleRenderElement,
};
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Physical, Point, Rectangle, Scale, Transform};
use std::fmt;

/// Where the strips' ids start: above the namespaces other wrappers use.
const NAMESPACE_BASE: usize = 64;

/// What turns an element into strip `index`, all in the output's physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripPlan {
    /// Which strip of the window this is (it keeps the strip's id steady from frame to frame).
    pub index: usize,
    /// The point the element is rescaled about: the window's top-left.
    pub origin: Point<i32, Physical>,
    /// The horizontal and vertical scale of this strip.
    pub scale: (f64, f64),
    /// The band to keep, in the rescaled element's space.
    pub band: Rectangle<i32, Physical>,
    /// How far the band moves to land where it is drawn.
    pub shift: Point<i32, Physical>,
}

/// `E` seen through a [`StripPlan`].
pub struct StripElement<E> {
    id: Id,
    inner: RelocateRenderElement<CropRenderElement<RescaleRenderElement<E>>>,
}

impl<E> fmt::Debug for StripElement<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StripElement")
            .field("id", &self.id)
            .finish()
    }
}

impl<E: Element> StripElement<E> {
    /// The strip `plan` makes of `element` at the output scale `output_scale`; `None` when the
    /// element has nothing in the band. Its id is the element's in a namespace of the strip's
    /// own, so the strips of one element are not one element to the damage tracker.
    pub fn new(element: E, output_scale: f64, plan: &StripPlan) -> Option<StripElement<E>> {
        let id = element.id().namespaced(NAMESPACE_BASE + plan.index);
        let scaled =
            RescaleRenderElement::from_element(element, plan.origin, Scale::from(plan.scale));
        let cropped = CropRenderElement::from_element(scaled, output_scale, plan.band)?;
        let inner = RelocateRenderElement::from_element(cropped, plan.shift, Relocate::Relative);
        Some(StripElement { id, inner })
    }
}

impl<E: Element> Element for StripElement<E> {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.inner.current_commit()
    }

    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        self.inner.location(scale)
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        self.inner.src()
    }

    fn transform(&self) -> Transform {
        self.inner.transform()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.inner.geometry(scale)
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        self.inner.damage_since(scale, commit)
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        self.inner.opaque_regions(scale)
    }

    fn alpha(&self) -> f32 {
        self.inner.alpha()
    }

    fn kind(&self) -> Kind {
        self.inner.kind()
    }

    fn is_framebuffer_effect(&self) -> bool {
        self.inner.is_framebuffer_effect()
    }
}

impl<R: Renderer, E: RenderElement<R>> RenderElement<R> for StripElement<E> {
    fn draw(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), R::Error> {
        self.inner
            .draw(frame, src, dst, damage, opaque_regions, cache)
    }

    fn underlying_storage(&self, renderer: &mut R) -> Option<UnderlyingStorage<'_>> {
        self.inner.underlying_storage(renderer)
    }

    fn capture_framebuffer(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), R::Error> {
        self.inner.capture_framebuffer(frame, src, dst, cache)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{Rgba8, solid_rect};
    use casement_core::geometry::PhysicalRect;
    use smithay::backend::renderer::element::solid::SolidColorRenderElement;

    fn solid() -> SolidColorRenderElement {
        let area = PhysicalRect {
            x: 100,
            y: 100,
            w: 200,
            h: 100,
        };
        let red = Rgba8 {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        solid_rect(Id::new(), area, red, CommitCounter::default())
    }

    fn plan(index: usize, band: Rectangle<i32, Physical>) -> StripPlan {
        StripPlan {
            index,
            origin: (100, 100).into(),
            scale: (0.5, 1.0),
            band,
            shift: (10, 20).into(),
        }
    }

    #[test]
    fn a_strip_lands_where_its_plan_says() {
        let band = Rectangle::new((100, 100).into(), (100, 50).into());
        let strip = StripElement::new(solid(), 1.0, &plan(3, band)).expect("the band overlaps");
        // half as wide about the origin, the top half kept, then moved by (10, 20)
        let want = Rectangle::new((110, 120).into(), (100, 50).into());
        assert_eq!(strip.geometry(Scale::from(1.0)), want);
    }

    #[test]
    fn a_band_the_element_does_not_reach_makes_no_strip() {
        let band = Rectangle::new((100, 500).into(), (100, 50).into());
        assert!(StripElement::new(solid(), 1.0, &plan(0, band)).is_none());
    }

    #[test]
    fn the_strips_of_one_element_are_different_elements() {
        let band = Rectangle::new((100, 100).into(), (100, 50).into());
        let element = solid();
        let id = element.id().clone();
        let one = StripElement::new(solid(), 1.0, &plan(1, band)).expect("overlap");
        let two = StripElement::new(solid(), 1.0, &plan(2, band)).expect("overlap");
        assert_ne!(one.id(), two.id());
        assert_ne!(one.id(), &id);
    }
}
