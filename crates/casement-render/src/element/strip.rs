//! One horizontal strip of an element, drawn somewhere else at another size: the piece the
//! genie is made of. The element is rescaled about the strip's origin, cropped to the strip's
//! band and moved to where the band is drawn, all by smithay's own wrappers, so damage, opaque
//! regions and the source rectangle follow from them.

use casement_core::geometry::PhysicalRect;
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
#[non_exhaustive]
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
    /// The rows of the window at rest this strip shows (top, bottom), in physical pixels from
    /// the output's top: what the clip and the blur ask to know which strips they touch.
    pub source: (f64, f64),
    /// How much of the band's first and last column the window covers (0 to 1): the band is
    /// the whole columns the strip touches, and its edge columns are partly covered where the
    /// edge falls inside them (`soft_edge.rs`).
    pub edges: (f64, f64),
}

/// How far to reach past a strip's band, in physical pixels of the window at rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Margins {
    /// Past the left and right edges.
    pub side: i32,
    /// Above the band.
    pub top: i32,
    /// Below the band.
    pub bottom: i32,
}

impl StripPlan {
    /// The same strip with its band reaching `by` farther (at rest; scaled here by the strip's
    /// own scale): a shadow's halo is outside the window's rectangle, so the strips that carry
    /// it are wider than the rows they cut, and the top and bottom strips reach past the
    /// window. The band moves with the shift, so the halo lands round the strip.
    pub fn widened(self, by: Margins) -> StripPlan {
        let grow = |n: i32, scale: f64| (f64::from(n) * scale).round() as i32;
        let side = grow(by.side, self.scale.0);
        let top = grow(by.top, self.scale.1);
        let bottom = grow(by.bottom, self.scale.1);
        StripPlan {
            band: Rectangle::new(
                (self.band.loc.x - side, self.band.loc.y - top).into(),
                (self.band.size.w + 2 * side, self.band.size.h + top + bottom).into(),
            ),
            ..self
        }
    }

    /// Where the rows of `rect` (a rectangle of the window at rest, output pixels) that this
    /// strip shows are drawn; `None` when it shows none of them. The rows are held to the
    /// strip's own drawn band, so two strips never claim the same pixel.
    pub fn mapped(&self, rect: PhysicalRect) -> Option<PhysicalRect> {
        let top = f64::from(rect.y).max(self.source.0);
        let bottom = (f64::from(rect.y) + f64::from(rect.h)).min(self.source.1);
        if bottom <= top {
            return None;
        }
        let across = |at: f64, by: f64, around: i32, shift: i32| {
            let around = f64::from(around);
            (around + (at - around) * by + f64::from(shift)).round() as i32
        };
        let (left, right) = (f64::from(rect.x), f64::from(rect.x) + f64::from(rect.w));
        let x0 = across(left, self.scale.0, self.origin.x, self.shift.x);
        let x1 = across(right, self.scale.0, self.origin.x, self.shift.x);
        let drawn = (self.band.loc.y + self.shift.y, self.band.size.h);
        let y0 = across(top, self.scale.1, self.origin.y, self.shift.y).max(drawn.0);
        let y1 = across(bottom, self.scale.1, self.origin.y, self.shift.y).min(drawn.0 + drawn.1);
        let side = |n: i32| u32::try_from(n).ok().filter(|n| *n > 0);
        Some(PhysicalRect {
            x: x0,
            y: y0,
            w: side(x1 - x0)?,
            h: side(y1 - y0)?,
        })
    }
}

/// The largest translation pixman's 16.16 fixed-point matrix holds, with half of it kept as a
/// margin: a genie edge column 1 pixel wide at x 1971 taking 15.4 source columns from x 1584
/// (31 937 by the sum below) still failed the whole frame, so the sum alone does not bound what
/// pixman computes.
const PIXMAN_FIXED_REACH: f64 = 16_000.0;

/// Whether pixman can draw `src` of a texture into `dst`: it maps the destination back to the
/// source through one fixed-point matrix whose translation is the destination's corner times
/// the squeeze, and refuses the whole frame when that overflows. A genie strip squeezed to a
/// pixel or two near the dock does (341 source columns into 1 at x 1440); such a strip covers
/// next to nothing, so the plain renderers skip it and draw the rest of the flight.
pub(crate) fn pixman_can_draw(
    src: Rectangle<f64, BufferCoords>,
    dst: Rectangle<i32, Physical>,
) -> bool {
    let fits = |corner: i32, from: f64, to: i32, offset: f64| {
        to > 0
            && (f64::from(corner).abs() * from / f64::from(to) + offset.abs()) < PIXMAN_FIXED_REACH
    };
    fits(dst.loc.x, src.size.w, dst.size.w, src.loc.x)
        && fits(dst.loc.y, src.size.h, dst.size.h, src.loc.y)
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
            source: (100.0, 150.0),
            edges: (1.0, 1.0),
        }
    }

    #[test]
    fn pixman_draws_a_strip_only_while_its_matrix_fits() {
        let src =
            |w: f64, h: f64| Rectangle::<f64, BufferCoords>::new((0.0, 0.0).into(), (w, h).into());
        let dst = |x: i32, y: i32, w: i32, h: i32| {
            Rectangle::<i32, Physical>::new((x, y).into(), (w, h).into())
        };
        // name, source, destination, drawable
        let cases = [
            ("at rest", src(800.0, 13.0), dst(200, 176, 800, 13), true),
            ("halved", src(800.0, 13.0), dst(3000, 2000, 400, 13), true),
            (
                "squeezed by the dock",
                src(341.0, 13.1),
                dst(1438, 1615, 1, 2),
                false,
            ),
            (
                "seven wide at 1448",
                src(341.0, 136.4),
                dst(1448, 1609, 7, 24),
                false,
            ),
            ("nothing wide", src(10.0, 10.0), dst(0, 0, 0, 10), false),
        ];
        for (name, src, dst, want) in cases {
            assert_eq!(pixman_can_draw(src, dst), want, "{name}");
        }
    }

    #[test]
    fn a_widened_band_reaches_out_by_the_strips_own_scale() {
        let band = Rectangle::new((100, 100).into(), (100, 50).into());
        let by = Margins {
            side: 8,
            top: 10,
            bottom: 0,
        };
        let wide = plan(0, band).widened(by);
        // scale (0.5, 1.0): 4 px each side, 10 px above; the shift is untouched
        assert_eq!(wide.band, Rectangle::new((96, 90).into(), (108, 60).into()));
        assert_eq!(wide.shift, plan(0, band).shift);
        assert_eq!(plan(0, band).widened(Margins::default()), plan(0, band));
    }

    #[test]
    fn a_rows_rectangle_lands_in_the_strips_band_and_nowhere_else() {
        let band = Rectangle::new((100, 100).into(), (100, 50).into());
        let plan = plan(0, band);
        // (x, y, w, h) at rest -> drawn; the strip shows rows 100..150, is halved across about
        // x 100 and moved by (10, 20)
        type Rect = (i32, i32, u32, u32);
        let cases: &[(&str, Rect, Option<Rect>)] = &[
            ("whole", (100, 100, 200, 50), Some((110, 120, 100, 50))),
            ("a part", (120, 120, 40, 10), Some((120, 140, 20, 10))),
            (
                "cut at the bottom",
                (100, 140, 200, 40),
                Some((110, 160, 100, 10)),
            ),
            ("above", (100, 0, 200, 100), None),
            ("below", (100, 150, 200, 10), None),
        ];
        for (name, (x, y, w, h), want) in cases {
            let got = plan.mapped(PhysicalRect {
                x: *x,
                y: *y,
                w: *w,
                h: *h,
            });
            let want = want.map(|(x, y, w, h)| PhysicalRect { x, y, w, h });
            assert_eq!(got, want, "{name}");
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
