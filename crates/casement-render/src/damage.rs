//! Drawing one output's elements through smithay's damage tracker.

use crate::compositor::{Compositor, EffectSupport};
use crate::element::{CasementElement, Rgba8};
use crate::scene::convert::ours;
use casement_core::geometry::PhysicalRect;
use smithay::backend::renderer::damage::{Error as DamageError, OutputDamageTracker};
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement, UnderlyingStorage};
use smithay::backend::renderer::utils::{CommitCounter, DamageSet, OpaqueRegions};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Physical, Point, Rectangle, Scale, Transform};

/// What one render produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// The damaged area, in the output's physical pixels (empty when nothing changed).
    pub damage: Vec<PhysicalRect>,
    /// Whether the frame is finished or waits on a fence.
    pub sync: RenderSync,
}

/// Whether a rendered frame is complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderSync {
    /// The frame is complete.
    Done,
    /// The GPU signals a fence when it is.
    Fence,
}

/// Why rendering failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    /// The renderer refused.
    #[error("renderer: {0}")]
    Renderer(String),
    /// The output has nothing to render into.
    #[error("output has no target")]
    NoTarget,
}

/// What every output shows where no element draws: opaque black.
const CLEAR: Rgba8 = Rgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

/// Draws `elements` (front first, smithay's order) into `target` with `tracker`, which knows
/// the output's mode, scale and transform. `age` is the target's buffer age (0 when unknown:
/// everything is redrawn). Waits for nothing: [`Rendered::sync`] says whether a fence is
/// pending. Where no element draws, the output is opaque black.
pub fn render_output<R: Compositor>(
    renderer: &mut R,
    target: &mut R::Framebuffer<'_>,
    tracker: &mut OutputDamageTracker,
    elements: &[CasementElement<R>],
    age: usize,
) -> Result<Rendered, RenderError> {
    let effects = renderer.effects();
    let routed: Vec<Routed<'_, R>> = elements
        .iter()
        .map(|element| Routed { element, effects })
        .collect();
    let result = tracker
        .render_output(renderer, target, age, &routed, CLEAR.color32f())
        .map_err(|error| match error {
            DamageError::Rendering(error) => RenderError::Renderer(error.to_string()),
            DamageError::OutputNoMode(_) => RenderError::NoTarget,
        })?;
    let damage = result
        .damage
        .map(|rects| rects.iter().copied().map(ours).collect())
        .unwrap_or_default();
    let sync = match result.sync.is_reached() {
        true => RenderSync::Done,
        false => RenderSync::Fence,
    };
    Ok(Rendered { damage, sync })
}

/// One element on its way to the tracker: effect variants go through the renderer's
/// [`Compositor`] hooks (shaders on GLES, stand-ins elsewhere), everything else as it is.
struct Routed<'a, R: Compositor> {
    element: &'a CasementElement<R>,
    effects: EffectSupport,
}

impl<R: Compositor> Element for Routed<'_, R> {
    fn id(&self) -> &Id {
        self.element.id()
    }

    fn current_commit(&self) -> CommitCounter {
        self.element.current_commit()
    }

    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        self.element.location(scale)
    }

    fn src(&self) -> Rectangle<f64, BufferCoords> {
        self.element.src()
    }

    fn transform(&self) -> Transform {
        self.element.transform()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.element.geometry(scale)
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        self.element.damage_since(scale, commit)
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        self.element.opaque_regions(scale)
    }

    fn alpha(&self) -> f32 {
        self.element.alpha()
    }

    fn kind(&self) -> Kind {
        self.element.kind()
    }

    /// A blur reads the framebuffer only where shaders run; its stand-in reads nothing.
    fn is_framebuffer_effect(&self) -> bool {
        match (self.element, self.effects) {
            (CasementElement::Blur(_), EffectSupport::Plain) => false,
            (element, _) => element.is_framebuffer_effect(),
        }
    }
}

impl<R: Compositor> RenderElement<R> for Routed<'_, R> {
    fn draw(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&UserDataMap>,
    ) -> Result<(), R::Error> {
        match self.element {
            CasementElement::Blur(blur) => R::draw_blur_v0(frame, blur, dst, damage, cache),
            CasementElement::Clip(clip) => {
                R::draw_rounded_clip(frame, clip, src, dst, damage, opaque_regions, cache)
            }
            element => element.draw(frame, src, dst, damage, opaque_regions, cache),
        }
    }

    fn underlying_storage(&self, renderer: &mut R) -> Option<UnderlyingStorage<'_>> {
        self.element.underlying_storage(renderer)
    }

    fn capture_framebuffer(
        &self,
        frame: &mut R::Frame<'_, '_>,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        cache: &UserDataMap,
    ) -> Result<(), R::Error> {
        match self.element {
            CasementElement::Blur(_) => R::capture_blur_v0(frame, dst, cache),
            element => element.capture_framebuffer(frame, src, dst, cache),
        }
    }
}
