//! One element held by several wrappers: the genie draws the same surface once per strip, and
//! smithay's elements are not `Clone`.

use smithay::backend::renderer::Renderer;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Physical, Point, Rectangle, Scale, Transform};
use std::fmt;
use std::rc::Rc;

/// `E`, shared: every clone is the same element.
pub struct SharedElement<E>(Rc<E>);

impl<E> SharedElement<E> {
    /// `element`, ready to be shared.
    pub fn new(element: E) -> SharedElement<E> {
        SharedElement(Rc::new(element))
    }

    /// The element all the clones share.
    pub fn get(&self) -> &E {
        &self.0
    }
}

impl<E> Clone for SharedElement<E> {
    fn clone(&self) -> SharedElement<E> {
        SharedElement(Rc::clone(&self.0))
    }
}

impl<E> fmt::Debug for SharedElement<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedElement").finish()
    }
}

impl<E: Element> Element for SharedElement<E> {
    fn id(&self) -> &Id {
        self.0.id()
    }

    fn current_commit(&self) -> CommitCounter {
        self.0.current_commit()
    }

    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        self.0.location(scale)
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        self.0.src()
    }

    fn transform(&self) -> Transform {
        self.0.transform()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.0.geometry(scale)
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        self.0.damage_since(scale, commit)
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        self.0.opaque_regions(scale)
    }

    fn alpha(&self) -> f32 {
        self.0.alpha()
    }

    fn kind(&self) -> Kind {
        self.0.kind()
    }

    fn is_framebuffer_effect(&self) -> bool {
        self.0.is_framebuffer_effect()
    }
}

impl<R: Renderer, E: RenderElement<R>> RenderElement<R> for SharedElement<E> {
    fn draw(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), R::Error> {
        self.0.draw(frame, src, dst, damage, opaque_regions, cache)
    }

    fn underlying_storage(&self, renderer: &mut R) -> Option<UnderlyingStorage<'_>> {
        self.0.underlying_storage(renderer)
    }

    fn capture_framebuffer(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), R::Error> {
        self.0.capture_framebuffer(frame, src, dst, cache)
    }
}
