//! What a frame is made of: the sampled motion, the layers on an output, and the elements the
//! renderer builds from them, for each renderer the headless backend can run.

use crate::state::State;
use casement_backend::headless::FrameSource;
use casement_core::geometry::{LogicalPoint, SubPx};
use casement_core::layer::{LayerBook, LayerLevel};
use casement_core::time::Mono;
use casement_core::{LayerKey, OutputKey, WindowId};
use casement_motion::{
    CommittedSizes, FrameScene, OutputFrame, StripOffset, WindowLook, WindowLooks, sample,
};
use casement_render::{
    CasementElement, ChromeLayer, Compositor, LayerDraw, OutputView, TextureBook, output_elements,
};
use casement_wayland::layer_shell::layer_area;
use casement_wayland::seat::{Candidate, SurfaceAt, surface_under};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::pixman::PixmanRenderer;
#[cfg(feature = "testing")]
use smithay::backend::renderer::test::DummyRenderer;

/// The frame the model's windows are in at `now`: motion sampled, each window's committed
/// size from the surface book, its look from the model.
pub(crate) fn sample_scene_at(state: &State, now: Mono) -> FrameScene {
    let model = &state.core.model;
    let book = &state.render.textures;
    let committed = CommittedSizes(
        model
            .windows()
            .keys()
            .filter_map(|id| book.window(id).map(|entry| (*id, entry.committed)))
            .collect(),
    );
    let looks = WindowLooks(
        model
            .windows()
            .iter()
            .map(|(id, window)| {
                let look = WindowLook {
                    emphasis: window.emphasis,
                    frame: window.hints.frame,
                };
                (*id, look)
            })
            .collect(),
    );
    sample(&state.core.motion, &committed, &looks, now).0
}

/// An output's frame with nothing on it (no window moves there).
pub(crate) fn empty_frame(output: &OutputKey) -> OutputFrame {
    OutputFrame {
        output: output.clone(),
        strip: StripOffset(SubPx(0)),
        windows: Vec::new(),
    }
}

/// The layer surfaces on `output`, by level, bottom of each level first (key order).
pub(crate) fn layers_on(layers: &LayerBook, output: &OutputKey) -> LayerDraw {
    layers
        .iter()
        .filter(|(_, spec)| spec.output == *output)
        .fold(LayerDraw::default(), |mut draw, (key, spec)| {
            let level = match spec.level {
                LayerLevel::Background => &mut draw.background,
                LayerLevel::Bottom => &mut draw.bottom,
                LayerLevel::Top => &mut draw.top,
                LayerLevel::Overlay => &mut draw.overlay,
            };
            level.push(*key);
            draw
        })
}

/// One output's frame, ready to be turned into elements for whichever renderer draws it.
pub(crate) struct SceneSource<'a> {
    pub(crate) frame: &'a OutputFrame,
    pub(crate) view: &'a OutputView,
    pub(crate) layers: &'a LayerDraw,
    pub(crate) book: &'a TextureBook,
    pub(crate) chrome: &'a ChromeLayer,
}

impl SceneSource<'_> {
    /// The elements, front first, for `renderer`.
    fn elements<R: Compositor>(&self, renderer: &mut R) -> Vec<CasementElement<R>> {
        output_elements(
            self.frame,
            self.view,
            self.layers,
            self.book,
            self.chrome,
            renderer,
        )
    }
}

impl FrameSource for SceneSource<'_> {
    fn pixman(&self, renderer: &mut PixmanRenderer) -> Vec<CasementElement<PixmanRenderer>> {
        self.elements(renderer)
    }

    fn gles(&self, renderer: &mut GlesRenderer) -> Vec<CasementElement<GlesRenderer>> {
        self.elements(renderer)
    }

    #[cfg(feature = "testing")]
    fn dummy(&self, renderer: &mut DummyRenderer) -> Vec<CasementElement<DummyRenderer>> {
        self.elements(renderer)
    }
}

/// What a surface in the stack belongs to.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Hit {
    /// The lock surface of an output.
    Lock(OutputKey),
    /// A layer surface.
    Layer(LayerKey),
    /// A window.
    Window(WindowId),
}

/// Every drawn root surface, front to back: lock surfaces, overlay and top layers, windows by
/// stacking (the sampled frame's, top first), bottom and background layers; each with where
/// its top-left is in global logical pixels.
pub(super) fn stack(state: &State) -> Vec<(Hit, Candidate)> {
    let model = &state.core.model;
    let book = &state.render.textures;
    let locks = state
        .wl
        .session_lock
        .surfaces
        .iter()
        .filter_map(|(output, lock)| {
            let rect = model.outputs().0.get(output)?.config.logical_rect();
            Some((
                Hit::Lock(output.clone()),
                Candidate {
                    root: lock.wl_surface().clone(),
                    origin: rect.origin,
                },
            ))
        });
    let layer = |levels: &'static [LayerLevel]| {
        let mut found: Vec<(Hit, Candidate)> = model
            .layers()
            .iter()
            .filter(|(_, spec)| levels.contains(&spec.level))
            .filter_map(|(key, spec)| {
                let entry = book.layer(key)?;
                let full = model.outputs().0.get(&spec.output)?.config.logical_rect();
                let area = layer_area(spec, full, entry.committed);
                Some((
                    Hit::Layer(*key),
                    Candidate {
                        root: entry.root.clone(),
                        origin: area.origin,
                    },
                ))
            })
            .collect();
        found.reverse();
        found
    };
    let windows = sample_scene_at(state, state.clock.now())
        .outputs
        .into_iter()
        .flat_map(|frame| frame.windows.into_iter().rev())
        .filter_map(|placed| {
            let entry = book.window(&placed.window)?;
            Some((
                Hit::Window(placed.window),
                Candidate {
                    root: entry.root.clone(),
                    origin: LogicalPoint {
                        x: placed.at.x.round_logical(),
                        y: placed.at.y.round_logical(),
                    },
                },
            ))
        });
    locks
        .chain(layer(&[LayerLevel::Overlay, LayerLevel::Top]))
        .chain(windows)
        .chain(layer(&[LayerLevel::Bottom, LayerLevel::Background]))
        .collect()
}

/// The first surface in `stack` with input at `at`, and what it belongs to.
pub(super) fn hit_test(stack: &[(Hit, Candidate)], at: LogicalPoint) -> Option<(Hit, SurfaceAt)> {
    stack.iter().find_map(|(hit, candidate)| {
        surface_under(std::slice::from_ref(candidate), at).map(|found| (hit.clone(), found))
    })
}
