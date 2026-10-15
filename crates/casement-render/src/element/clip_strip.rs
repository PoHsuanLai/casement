//! A strip of a genie's client surface that the rounded clip reaches: the bottom strips, where
//! the window's corners are. The strip is drawn as any other (`strip.rs`); on GLES the renderer
//! draws its texture through the clip's shader instead, with the mask taken from the surface as
//! it stands at rest. The coverage is then a function of the window's own pixels, so the
//! corners are cut where the window's are and the strip carries the cut along, however it is
//! scaled and moved.

use super::rounded_clip::{ClipShape, Mask, draw_masked};
use super::shared::SharedElement;
use super::strip::{StripElement, StripPlan};
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement};
use smithay::backend::renderer::gles::{GlesError, GlesRenderer};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::backend::renderer::{ImportAll, Renderer, Texture};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Physical, Point, Rectangle, Scale, Transform};
use std::fmt;

/// The namespace that tells a clipped strip from the plain strip of the same surface.
const NAMESPACE: usize = 3;

/// The surface element a clipped strip shows.
pub type SharedSurface<R> = SharedElement<WaylandSurfaceRenderElement<R>>;

/// A strip of a surface, drawn through the rounded clip of `shape`.
pub struct ClipStripElement<R: Renderer> {
    id: Id,
    strip: StripElement<SharedSurface<R>>,
    surface: SharedSurface<R>,
    shape: ClipShape,
    scale: Scale<f64>,
}

impl<R: Renderer> fmt::Debug for ClipStripElement<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipStripElement")
            .field("id", &self.id)
            .field("shape", &self.shape)
            .finish()
    }
}

impl<R: Renderer + ImportAll> ClipStripElement<R> {
    /// The strip `plan` makes of `surface` at the output scale `output_scale`, clipped to
    /// `shape` (the window's rounded rectangle at rest); `None` when the band holds none of it.
    pub fn new(
        surface: SharedSurface<R>,
        output_scale: f64,
        plan: &StripPlan,
        shape: ClipShape,
    ) -> Option<ClipStripElement<R>> {
        let strip = StripElement::new(surface.clone(), output_scale, plan)?;
        Some(ClipStripElement {
            id: strip.id().namespaced(NAMESPACE),
            strip,
            surface,
            shape,
            scale: Scale::from(output_scale),
        })
    }

    /// The plain strip this one clips.
    pub fn strip(&self) -> &StripElement<SharedSurface<R>> {
        &self.strip
    }

    /// The mask: the surface's own rectangles at rest, and the shape.
    fn mask(&self) -> Mask {
        let surface = self.surface.get();
        Mask {
            shape: self.shape,
            src: surface.src(),
            dst: surface.geometry(self.scale),
        }
    }
}

impl<R: Renderer + ImportAll> Element for ClipStripElement<R> {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.strip.current_commit()
    }

    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        self.strip.location(scale)
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        self.strip.src()
    }

    fn transform(&self) -> Transform {
        self.strip.transform()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.strip.geometry(scale)
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        self.strip.damage_since(scale, commit)
    }

    /// None: the clip makes the pixels at the corners translucent, and a strip is small enough
    /// that the saving is not worth working out which of them are.
    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        let _ = scale;
        OpaqueRegions::default()
    }

    fn alpha(&self) -> f32 {
        self.strip.alpha()
    }

    fn kind(&self) -> Kind {
        self.strip.kind()
    }
}

/// As the plain strip; [`crate::Compositor::draw_clip_strip`] is what clips. It offers no
/// underlying storage (the default), so a clipped strip is never scanned out unclipped.
impl<R> RenderElement<R> for ClipStripElement<R>
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
        self.strip
            .draw(frame, src, dst, damage, opaque_regions, cache)
    }
}

impl ClipStripElement<GlesRenderer> {
    /// GLES: draws the strip's texture through the corner mask of the window at rest.
    pub(crate) fn draw_gles(
        &self,
        frame: &mut smithay::backend::renderer::gles::GlesFrame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        let surface = self.surface.get();
        draw_masked(
            frame,
            surface,
            self.mask(),
            (src, dst),
            (damage, opaque_regions),
            |frame| surface.draw(frame, src, dst, damage, opaque_regions, cache),
        )
    }
}
