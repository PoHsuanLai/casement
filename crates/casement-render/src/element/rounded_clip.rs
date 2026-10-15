//! A window's surface clipped at its two bottom corners (a Mac window is rounded on all four;
//! the titlebar rounds the top two itself). The element wraps a surface element of the window's
//! tree and says where the window's rounded rectangle is; on GLES the renderer draws the
//! surface's texture through a shader that masks outside it (`gles.rs`), elsewhere the surface
//! is drawn unclipped (FINDINGS, `wave-2026-10-15/clip`).
//!
//! Only windows casement decorates are clipped: one that draws its own frame draws its own
//! corners, a full-screen window has square ones, and popups are never wrapped.

mod gles;
mod shape;

pub(crate) use gles::{Mask, draw_masked};
pub use shape::ClipShape;

use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement};
use smithay::backend::renderer::gles::{GlesError, GlesFrame, GlesRenderer};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::backend::renderer::{ImportAll, Renderer, Texture};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Physical, Point, Rectangle, Scale, Transform};
use std::fmt;

/// The namespace that tells the clipped surface's element from the plain one (same resource).
const NAMESPACE: usize = 2;

/// A surface element drawn only inside `shape`.
pub struct RoundedClipElement<R: Renderer> {
    id: Id,
    surface: WaylandSurfaceRenderElement<R>,
    shape: ClipShape,
}

impl<R: Renderer> fmt::Debug for RoundedClipElement<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RoundedClipElement")
            .field("id", &self.id)
            .field("shape", &self.shape)
            .finish()
    }
}

impl<R: Renderer + ImportAll> RoundedClipElement<R> {
    /// `surface` clipped to `shape`. Its id is the surface's in a namespace of its own, so a
    /// window gaining or losing its clip is a new element to the damage tracker.
    pub fn new(surface: WaylandSurfaceRenderElement<R>, shape: ClipShape) -> RoundedClipElement<R> {
        RoundedClipElement {
            id: surface.id().namespaced(NAMESPACE),
            surface,
            shape,
        }
    }

    /// The surface element being clipped.
    pub fn surface(&self) -> &WaylandSurfaceRenderElement<R> {
        &self.surface
    }

    /// The shape it is clipped to.
    pub fn shape(&self) -> ClipShape {
        self.shape
    }
}

impl<R: Renderer + ImportAll> Element for RoundedClipElement<R> {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.surface.current_commit()
    }

    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        self.surface.location(scale)
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        self.surface.src()
    }

    fn transform(&self) -> Transform {
        self.surface.transform()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.surface.geometry(scale)
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        self.surface.damage_since(scale, commit)
    }

    /// The surface's opaque regions without the two corner squares, where the clip makes the
    /// pixels translucent (the damage tracker must blend there, not overwrite).
    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        let origin = self.surface.geometry(scale).loc;
        let squares = self
            .shape
            .corner_squares()
            .map(|square| Rectangle::new(square.loc - origin, square.size));
        self.surface
            .opaque_regions(scale)
            .iter()
            .flat_map(|region| region.subtract_rects(squares))
            .collect::<OpaqueRegions<_, _>>()
    }

    fn alpha(&self) -> f32 {
        self.surface.alpha()
    }

    fn kind(&self) -> Kind {
        self.surface.kind()
    }
}

/// As a plain surface; [`crate::Compositor::draw_rounded_clip`] is what clips. It offers no
/// underlying storage (the default), so a clipped window is never scanned out unclipped.
impl<R> RenderElement<R> for RoundedClipElement<R>
where
    R: Renderer + ImportAll,
    R::TextureId: Texture + 'static,
{
    fn draw(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), R::Error> {
        self.surface
            .draw(frame, src, dst, damage, opaque_regions, cache)
    }
}

impl RoundedClipElement<GlesRenderer> {
    /// GLES: draws the surface through the corner mask (see `gles`).
    pub(crate) fn draw_gles(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        gles::draw(frame, self, src, dst, damage, opaque_regions, cache)
    }
}
