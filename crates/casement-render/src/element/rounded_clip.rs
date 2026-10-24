//! A window's surface clipped at its two bottom corners (a Mac window is rounded on all four;
//! the titlebar rounds the top two itself), or at all four when there is no titlebar of ours. The element wraps a surface element of the window's
//! tree and says where the window's rounded rectangle is; on GLES the renderer draws the
//! surface's texture through a shader that masks outside it (`gles.rs`); every other draw
//! (Pixman, the dummy renderer, a multi-GPU copy, and the GLES surfaces the shader leaves out)
//! draws the surface in pieces by coverage (`bands.rs`; FINDINGS, `wave-2026-10-15/corners`).
//!
//! Only windows casement shapes are clipped: the ones it decorates (the bottom corners), and
//! the ones that draw their own frame but asked for casement's corners or said nothing (all four,
//! `Rounding::Frameless`). A window that said it draws its own corners, a full-screen window and
//! popups are not wrapped.

mod bands;
mod gles;
mod shape;

pub(crate) use gles::{Mask, bake_corners, draw_masked};
pub use shape::{ClipShape, Rounding};

use crate::compositor::Compositor;
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

/// How a window in a Scale flight is stretched: about the window geometry's corner, per axis.
/// smithay's own `RescaleRenderElement` does the same for an element that is not clipped; the
/// clip keeps the surface element it draws from, so it applies the same arithmetic itself.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Stretch {
    about: Point<i32, Physical>,
    ratio: Scale<f64>,
}

impl Stretch {
    /// `rest` (the surface as laid out at its real size) stretched about the corner.
    fn geometry(self, rest: Rectangle<i32, Physical>) -> Rectangle<i32, Physical> {
        let mut relative = rest;
        relative.loc -= self.about;
        let mut stretched: Rectangle<i32, Physical> =
            relative.to_f64().upscale(self.ratio).to_i32_round();
        stretched.loc += self.about;
        stretched
    }

    /// A region of the surface (relative to its own corner) as it is after the stretch.
    fn region(self, rect: Rectangle<i32, Physical>) -> Rectangle<i32, Physical> {
        rect.to_f64().upscale(self.ratio).to_i32_round()
    }

    /// A damaged rectangle of the surface (relative to its own corner) after the stretch.
    fn damage(self, rect: Rectangle<i32, Physical>) -> Rectangle<i32, Physical> {
        rect.to_f64().upscale(self.ratio).to_i32_up()
    }
}

/// A surface element drawn only inside `shape`.
pub struct RoundedClipElement<R: Renderer> {
    id: Id,
    surface: WaylandSurfaceRenderElement<R>,
    shape: ClipShape,
    stretch: Option<Stretch>,
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
            stretch: None,
        }
    }

    /// The same element stretched by `ratio` about `about` (the window geometry's corner, output
    /// pixels): the surface is laid out at its real size and drawn at the stretched one, as a
    /// window in a Scale flight is. `shape` is then the stretched window's.
    pub fn stretched(
        self,
        about: Point<i32, Physical>,
        ratio: (f64, f64),
    ) -> RoundedClipElement<R> {
        RoundedClipElement {
            stretch: Some(Stretch {
                about,
                ratio: Scale::from(ratio),
            }),
            ..self
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
        self.geometry(scale).loc
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        self.surface.src()
    }

    fn transform(&self) -> Transform {
        self.surface.transform()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        let rest = self.surface.geometry(scale);
        self.stretch.map_or(rest, |stretch| stretch.geometry(rest))
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        let damage = self.surface.damage_since(scale, commit);
        match self.stretch {
            Some(stretch) => damage
                .into_iter()
                .map(|rect| stretch.damage(rect))
                .collect::<DamageSet<_, _>>(),
            None => damage,
        }
    }

    /// The surface's opaque regions without the two corner squares, where the clip makes the
    /// pixels translucent (the damage tracker must blend there, not overwrite).
    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        let origin = self.geometry(scale).loc;
        let squares = self
            .shape
            .corner_squares()
            .into_iter()
            .map(|square| Rectangle::new(square.loc - origin, square.size))
            .collect::<Vec<_>>();
        self.surface
            .opaque_regions(scale)
            .iter()
            .map(|region| match self.stretch {
                Some(stretch) => stretch.region(*region),
                None => *region,
            })
            .flat_map(|region| region.subtract_rects(squares.iter().copied()))
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

impl<R: Compositor> RoundedClipElement<R> {
    /// Draws the surface clipped to the shape on any renderer, from pieces of the surface
    /// weighted by coverage (see `bands`). `damage` is relative to `dst`.
    pub(crate) fn draw_banded(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), R::Error> {
        bands::draw(frame, self, src, dst, damage, opaque_regions)
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
