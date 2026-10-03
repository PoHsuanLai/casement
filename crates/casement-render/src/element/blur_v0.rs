//! The P0 behind-surface blur (ext-background-effect-v1): a single downsample-and-blur of what
//! lies under a surface's blur region. On renderers without effect shaders
//! ([`crate::EffectSupport::Plain`]) it draws its solid stand-in. P3 replaces it with the dual
//! Kawase pass (`pass/blur_kawase.rs`).

use super::solid::{Rgba8, fingerprint};
use crate::scene::OutputView;
use crate::scene::convert::{on_output, ours, physical};
use casement_core::geometry::{PhysicalRect, SubPoint, SubPx};
use smithay::backend::renderer::element::{Element, Id, RenderElement};
use smithay::backend::renderer::gles::{GlesError, GlesFrame};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::backend::renderer::{Frame, Renderer};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Logical, Physical, Rectangle, Scale};
use smithay::wayland::background_effect::BackgroundEffectSurfaceCachedState;
use smithay::wayland::compositor::{RectangleKind, RegionAttributes, with_states};

mod gles;

/// The stand-in: a light, mostly opaque wash (premultiplied 0.96 white at 0.85), the quiet
/// look of a material when there is nothing to blur with.
pub(crate) const STAND_IN: Rgba8 = Rgba8 {
    r: 208,
    g: 208,
    b: 208,
    a: 217,
};

/// The namespace that tells a surface's blur from the surface's own element (same resource).
const NAMESPACE: usize = 1;

/// The blur v0 a surface asked for through ext-background-effect (its committed blur region,
/// surface-local), behind `root` whose top-left is at `origin` (global sub-pixels) on `view`;
/// none without a region or with an empty one.
pub(crate) fn behind(
    root: &WlSurface,
    origin: SubPoint,
    view: &OutputView,
) -> Option<BlurV0Element> {
    let region = with_states(root, |states| {
        states
            .cached_state
            .get::<BackgroundEffectSurfaceCachedState>()
            .current()
            .blur_region
            .clone()
    })?;
    let rects: Vec<PhysicalRect> = disjoint(&region)
        .into_iter()
        .map(|rect| placed(rect, origin, view))
        .filter(|rect| rect.w > 0 && rect.h > 0)
        .collect();
    let id = Id::from_wayland_resource(root).namespaced(NAMESPACE);
    (!rects.is_empty()).then(|| BlurV0Element::new(id, rects, STAND_IN))
}

/// A region's rectangles, added and subtracted in order, as rectangles that do not overlap
/// (a translucent stand-in drawn twice would show the overlap).
fn disjoint(region: &RegionAttributes) -> Vec<Rectangle<i32, Logical>> {
    region
        .rects
        .iter()
        .fold(Vec::new(), |kept, (kind, rect)| match kind {
            RectangleKind::Add => {
                let fresh = rect.subtract_rects(kept.iter().copied());
                kept.into_iter().chain(fresh).collect()
            }
            RectangleKind::Subtract => kept
                .into_iter()
                .flat_map(|old| old.subtract_rect(*rect))
                .collect(),
        })
}

/// A surface-local rectangle of a surface at `origin`, in `view`'s physical pixels (each edge
/// rounded, so neighbours stay touching).
fn placed(rect: Rectangle<i32, Logical>, origin: SubPoint, view: &OutputView) -> PhysicalRect {
    let corner = |dx: i32, dy: i32| {
        on_output(
            SubPoint {
                x: SubPx(origin.x.0.saturating_add(SubPx::of_logical(dx).0)),
                y: SubPx(origin.y.0.saturating_add(SubPx::of_logical(dy).0)),
            },
            view,
        )
    };
    let start = corner(rect.loc.x, rect.loc.y);
    let end = corner(
        rect.loc.x.saturating_add(rect.size.w),
        rect.loc.y.saturating_add(rect.size.h),
    );
    ours(Rectangle::new(start, (end - start).to_size()))
}

/// A blur of the framebuffer under `region`, tinted with the stand-in colour when shaders are
/// off.
///
/// The element alone draws the stand-in on any renderer; [`crate::render_output`] routes it
/// through [`crate::Compositor`], where GLES captures and blurs the framebuffer instead.
#[derive(Debug, Clone)]
pub struct BlurV0Element {
    id: Id,
    region: Vec<PhysicalRect>,
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

    /// The smallest rectangle holding the region (empty for an empty region).
    fn bounds(&self) -> Rectangle<i32, Physical> {
        self.region
            .iter()
            .map(|rect| physical(*rect))
            .filter(|rect| !rect.is_empty())
            .reduce(Rectangle::merge)
            .unwrap_or_default()
    }

    /// The region relative to the element's top-left.
    fn local_region(&self) -> Vec<Rectangle<i32, Physical>> {
        let origin = self.bounds().loc;
        self.region
            .iter()
            .map(|rect| physical(*rect))
            .filter(|rect| !rect.is_empty())
            .map(|rect| Rectangle::new(rect.loc - origin, rect.size))
            .collect()
    }

    /// `damage` (relative to the element) cut down to the region.
    pub(crate) fn clip(
        &self,
        damage: &[Rectangle<i32, Physical>],
    ) -> Vec<Rectangle<i32, Physical>> {
        let region = self.local_region();
        damage
            .iter()
            .flat_map(|d| region.iter().filter_map(|r| r.intersection(*d)))
            .collect()
    }

    /// GLES: copies what lies under `dst` into `cache`, downsampled (see `gles`).
    pub(crate) fn capture_gles(
        frame: &mut GlesFrame<'_, '_>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), GlesError> {
        gles::capture(frame, dst, cache)
    }

    /// GLES: draws the blurred copy over the region (the stand-in before any capture).
    pub(crate) fn draw_gles(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), GlesError> {
        gles::draw(frame, self, dst, damage, cache)
    }

    /// Draws the stand-in: the region in the stand-in colour.
    pub(crate) fn draw_stand_in<F: Frame>(
        &self,
        frame: &mut F,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), F::Error> {
        let clipped = self.clip(damage);
        match clipped.is_empty() {
            true => Ok(()),
            false => frame.draw_solid(dst, &clipped, self.stand_in.color32f()),
        }
    }
}

impl Element for BlurV0Element {
    fn id(&self) -> &Id {
        &self.id
    }

    /// What the element itself draws is the stand-in over the region, so the commit follows
    /// the region's shape and the colour (a new framebuffer under it is smithay's to notice:
    /// a framebuffer effect is captured again whenever what lies behind it changes).
    fn current_commit(&self) -> CommitCounter {
        let colour = [
            self.stand_in.r,
            self.stand_in.g,
            self.stand_in.b,
            self.stand_in.a,
        ];
        let shape = self.local_region().into_iter().flat_map(|rect| {
            [rect.loc.x, rect.loc.y, rect.size.w, rect.size.h]
                .into_iter()
                .flat_map(i32::to_le_bytes)
        });
        fingerprint(colour.into_iter().chain(shape))
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        let size = self.bounds().size;
        Rectangle::from_size((f64::from(size.w), f64::from(size.h)).into())
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        let _ = scale;
        self.bounds()
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        let _ = scale;
        match commit == Some(self.current_commit()) {
            true => DamageSet::default(),
            false => DamageSet::from_slice(&[Rectangle::from_size(self.bounds().size)]),
        }
    }

    /// Opaque where the region is, when the stand-in is: the GLES blur of an opaque frame is
    /// opaque too.
    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        let _ = scale;
        match self.stand_in.is_opaque() {
            true => OpaqueRegions::from_slice(&self.local_region()),
            false => OpaqueRegions::default(),
        }
    }

    fn is_framebuffer_effect(&self) -> bool {
        true
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
        let _ = (src, opaque_regions, cache);
        self.draw_stand_in(frame, dst, damage)
    }

    /// The stand-in needs nothing from the framebuffer.
    fn capture_framebuffer(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), R::Error> {
        let _ = (frame, src, dst, cache);
        Ok(())
    }
}
