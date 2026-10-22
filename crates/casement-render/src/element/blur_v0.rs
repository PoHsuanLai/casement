//! The behind-surface blur (ext-background-effect-v1): a dual-Kawase blur of what lies under a
//! surface's blur region (`pass/blur_kawase.rs`), saturated and clipped to the region's
//! rectangles. The element's geometry is the region grown by the blur's reach, so damage that
//! close to the region blurs again; nothing is drawn outside the region. On renderers without
//! effect shaders ([`crate::EffectSupport::Plain`]) it draws its solid stand-in. The type keeps
//! its P0 name so the elements that hold it need not change.

use super::reduced::BlurFinish;
use super::solid::{Rgba8, fingerprint};
use crate::pass::{BlurLook, KawasePlan};
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
/// surface-local), behind `root` whose top-left is at `origin` (global sub-pixels) on `view`,
/// stretched by `ratio` about its top-left (a window in a Scale flight) and drawn at the first of
/// `(alpha, look)` (the surface's, so the blur fades with it); none without a region or with an
/// empty one.
pub(crate) fn behind_scaled(
    root: &WlSurface,
    origin: SubPoint,
    ratio: (f64, f64),
    (alpha, look): (f32, BlurLook),
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
        .map(|rect| placed(rect, origin, ratio, view))
        .filter(|rect| rect.w > 0 && rect.h > 0)
        .collect();
    let id = Id::from_wayland_resource(root).namespaced(NAMESPACE);
    (!rects.is_empty()).then(|| {
        BlurV0Element::new(id, rects, STAND_IN)
            .with_alpha(alpha)
            .with_look(look, view.scale.as_f64())
    })
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

/// A surface-local rectangle of a surface at `origin` stretched by `ratio`, in `view`'s
/// physical pixels (each edge rounded, so neighbours stay touching).
fn placed(
    rect: Rectangle<i32, Logical>,
    origin: SubPoint,
    ratio: (f64, f64),
    view: &OutputView,
) -> PhysicalRect {
    let offset = |d: i32, by: f64| (f64::from(SubPx::of_logical(d).0) * by).round() as i32;
    let corner = |dx: i32, dy: i32| {
        on_output(
            SubPoint {
                x: SubPx(origin.x.0.saturating_add(offset(dx, ratio.0))),
                y: SubPx(origin.y.0.saturating_add(offset(dy, ratio.1))),
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
    alpha: f32,
    plan: KawasePlan,
    finish: BlurFinish,
}

impl BlurV0Element {
    /// A blur under `region` (output-local physical pixels), drawn as `stand_in` without
    /// shaders.
    pub fn new(id: Id, region: Vec<PhysicalRect>, stand_in: Rgba8) -> BlurV0Element {
        BlurV0Element {
            id,
            region,
            stand_in,
            alpha: 1.0,
            plan: BlurLook::MATERIAL.plan(1.0),
            finish: BlurFinish::Frosted,
        }
    }

    /// The same blur finished as `finish`: solid, it is not a blur at all but that colour, on
    /// every renderer (nothing is captured from the framebuffer).
    ///
    /// ```
    /// use casement_render::{BlurFinish, BlurV0Element, Rgba8};
    /// use smithay::backend::renderer::element::{Element, Id};
    ///
    /// let black = Rgba8 { r: 0, g: 0, b: 0, a: 255 };
    /// let blur = BlurV0Element::new(Id::new(), Vec::new(), black);
    /// assert!(blur.is_framebuffer_effect());
    /// let grey = Rgba8 { r: 122, g: 122, b: 122, a: 245 };
    /// assert!(!blur.with_finish(BlurFinish::Solid(grey)).is_framebuffer_effect());
    /// ```
    pub fn with_finish(self, finish: BlurFinish) -> BlurV0Element {
        BlurV0Element { finish, ..self }
    }

    /// Whether the blur is drawn or replaced by a solid colour.
    pub fn finish(&self) -> BlurFinish {
        self.finish
    }

    /// The colour the stand-in is drawn in: the one it was made with, or the solid one.
    pub fn stand_in(&self) -> Rgba8 {
        match self.finish {
            BlurFinish::Frosted => self.stand_in,
            BlurFinish::Solid(colour) => colour,
        }
    }

    /// The same blur as `look` asks on an output at `scale` (physical pixels per logical pixel).
    ///
    /// ```
    /// use casement_render::pass::BlurLook;
    /// use casement_render::{BlurV0Element, Rgba8};
    /// use smithay::backend::renderer::element::Id;
    ///
    /// let black = Rgba8 { r: 0, g: 0, b: 0, a: 255 };
    /// let blur = BlurV0Element::new(Id::new(), Vec::new(), black)
    ///     .with_look(BlurLook::MATERIAL, 2.0);
    /// assert_eq!(blur.plan().passes, 5);
    /// ```
    pub fn with_look(self, look: BlurLook, scale: f64) -> BlurV0Element {
        BlurV0Element {
            plan: look.plan(scale),
            ..self
        }
    }

    /// The passes this blur runs.
    pub fn plan(&self) -> &KawasePlan {
        &self.plan
    }

    /// The same blur drawn at `alpha` (0 to 1): a window that fades takes its blur with it.
    ///
    /// ```
    /// use casement_render::{BlurV0Element, Rgba8};
    /// use smithay::backend::renderer::element::Id;
    ///
    /// let black = Rgba8 { r: 0, g: 0, b: 0, a: 255 };
    /// let blur = BlurV0Element::new(Id::new(), Vec::new(), black).with_alpha(0.5);
    /// assert_eq!(blur.alpha_of(), 0.5);
    /// ```
    pub fn with_alpha(self, alpha: f32) -> BlurV0Element {
        BlurV0Element {
            alpha: alpha.clamp(0.0, 1.0),
            ..self
        }
    }

    /// How opaque the blur is drawn.
    pub fn alpha_of(&self) -> f32 {
        self.alpha
    }

    /// The same blur over the rectangles `map` makes of each of this one's (a window cut into
    /// strips has its blur cut the same way); the id, the stand-in and the alpha are kept.
    pub fn remapped<I: IntoIterator<Item = PhysicalRect>>(
        &self,
        map: impl Fn(PhysicalRect) -> I,
    ) -> BlurV0Element {
        BlurV0Element {
            id: self.id.clone(),
            region: self.region.iter().copied().flat_map(map).collect(),
            stand_in: self.stand_in,
            alpha: self.alpha,
            plan: self.plan,
            finish: self.finish,
        }
    }

    /// The rectangles it blurs under (output pixels).
    pub fn region(&self) -> &[PhysicalRect] {
        &self.region
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

    /// What the element covers: the region's bounds grown by the blur's reach (the blur reads
    /// that far beyond the region), so damage within reach captures the blur again.
    fn area(&self) -> Rectangle<i32, Physical> {
        let bounds = self.bounds();
        match bounds.is_empty() {
            true => bounds,
            false => self.plan.expand(bounds),
        }
    }

    /// The region relative to the element's top-left.
    fn local_region(&self) -> Vec<Rectangle<i32, Physical>> {
        let origin = self.area().loc;
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

    /// GLES: blurs what lies under `dst` into `cache` (see `pass::blur_kawase`).
    pub(crate) fn capture_gles(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), GlesError> {
        gles::capture(frame, self, dst, cache)
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
            false => frame.draw_solid(dst, &clipped, self.stand_in().faded(self.alpha)),
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
        let stand_in = self.stand_in();
        let colour = [stand_in.r, stand_in.g, stand_in.b, stand_in.a];
        let plan = &self.plan;
        let look = [
            plan.offset.to_bits(),
            plan.saturation.to_bits(),
            plan.noise.to_bits(),
            u32::from(plan.passes),
            u32::from_le_bytes([plan.tint.r, plan.tint.g, plan.tint.b, plan.tint.a]),
        ]
        .into_iter()
        .flat_map(u32::to_le_bytes);
        let shape = self.local_region().into_iter().flat_map(|rect| {
            [rect.loc.x, rect.loc.y, rect.size.w, rect.size.h]
                .into_iter()
                .flat_map(i32::to_le_bytes)
        });
        let alpha = self.alpha.to_bits().to_le_bytes();
        let finish = u8::from(self.finish != BlurFinish::Frosted);
        fingerprint(
            colour
                .into_iter()
                .chain(alpha)
                .chain(look)
                .chain(shape)
                .chain([finish]),
        )
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        let size = self.area().size;
        Rectangle::from_size((f64::from(size.w), f64::from(size.h)).into())
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        let _ = scale;
        self.area()
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        let _ = scale;
        match commit == Some(self.current_commit()) {
            true => DamageSet::default(),
            false => DamageSet::from_slice(&[Rectangle::from_size(self.area().size)]),
        }
    }

    /// Opaque where the region is, when the stand-in is: the GLES blur of an opaque frame is
    /// opaque too.
    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        let _ = scale;
        match self.stand_in().is_opaque() && self.alpha >= 1.0 {
            true => OpaqueRegions::from_slice(&self.local_region()),
            false => OpaqueRegions::default(),
        }
    }

    fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Only a blur that is drawn reads the framebuffer; reduced, its stand-in reads nothing.
    fn is_framebuffer_effect(&self) -> bool {
        self.finish == BlurFinish::Frosted
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
