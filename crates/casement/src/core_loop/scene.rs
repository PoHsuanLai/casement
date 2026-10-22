//! What a frame is made of: the sampled motion, the layers on an output, and the elements the
//! renderer builds from them, for each renderer the headless backend can run.

use crate::state::State;
use casement_backend::headless::FrameElements;
use casement_core::Model;
use casement_core::geometry::{LogicalPoint, LogicalRect, SubPx};
use casement_core::layer::{Arranged, BlurStrength, LayerLevel};
use casement_core::time::Mono;
use casement_core::window::WindowState;
use casement_core::{LayerKey, OutputKey, WindowId};
use casement_motion::{
    CommittedSizes, FrameScene, OutputFrame, StripOffset, WindowLook, WindowLooks, sample,
};
use casement_render::{
    CasementElement, ChromeLayer, Compositor, LayerDraw, OutputView, RowDraw, TextureBook,
    output_elements, popup_surfaces, surface_origin_logical,
};
use casement_wayland::seat::{Candidate, SurfaceAt, surface_under};
use std::collections::BTreeMap;

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

/// The row of Spaces on the output of `frame` at `now`, ready to draw: each window's column and
/// where the row is. While a swipe moves the row (the fingers down, or the finish after them) it
/// is the model's position, rubber band and ease-out included; otherwise it is the strip motion
/// sampled, which rests with the shown Space at the usable origin. None when no window lives on
/// a Space of the output.
pub(crate) fn row_of(state: &State, frame: &OutputFrame, now: Mono) -> Option<RowDraw> {
    let model = &state.core.model;
    let sampled = frame.strip.0;
    row_where(state, &frame.output, |usable| {
        match model.space_strip(now) {
            Some(strip) if strip.output == frame.output => strip.first_x(usable),
            _ => sampled,
        }
    })
}

/// The row the pointer meets on the output of `frame` at `now`: only a swipe that is moving the
/// row (the fingers down, or its finish) shifts it, as drawn. Otherwise the shown Space sits at
/// the usable origin, where the windows will be, whatever a keyboard or finished slide is
/// drawing mid-way: the pointer is tested against places, not against a flight.
fn hit_row_of(state: &State, frame: &OutputFrame, now: Mono) -> Option<RowDraw> {
    let model = &state.core.model;
    let output = &frame.output;
    let shown = model.shown_index(output).unwrap_or(0);
    row_where(state, output, |usable| match model.space_strip(now) {
        Some(strip) if strip.output == *output => strip.first_x(usable),
        _ => {
            let width = i32::try_from(usable.size.w).unwrap_or(i32::MAX);
            let behind = i64::from(SubPx::of_logical(width).0) * i64::from(shown);
            let at = i64::from(SubPx::of_logical(usable.origin.x).0) - behind;
            SubPx(i32::try_from(at).unwrap_or(i32::MIN))
        }
    })
}

/// The row of `output` with its first Space starting where `first_x` says for the usable area.
/// None when no window lives on a Space of the output.
fn row_where(
    state: &State,
    output: &OutputKey,
    first_x: impl FnOnce(LogicalRect) -> SubPx,
) -> Option<RowDraw> {
    let model = &state.core.model;
    let columns = model.row_columns(output);
    if columns.is_empty() {
        return None;
    }
    let usable = model
        .layers()
        .usable_area(output, model.outputs().0.get(output)?.config.logical_rect());
    let first = first_x(usable);
    let width = i32::try_from(usable.size.w).unwrap_or(i32::MAX);
    Some(RowDraw {
        offset: SubPx(first.0.saturating_sub(SubPx::of_logical(usable.origin.x).0)),
        page: SubPx::of_logical(width),
        columns,
    })
}

/// [`sample_scene_at`] as the pointer meets it (`hit_row_of`): each window in its Space's column,
/// the windows that are not on their output's picture left out.
pub(crate) fn drawn_scene_at(state: &State, now: Mono) -> FrameScene {
    let outputs = sample_scene_at(state, now)
        .outputs
        .into_iter()
        .map(|frame| {
            let area = state
                .core
                .model
                .outputs()
                .0
                .get(&frame.output)
                .map(|output| output.config.logical_rect());
            match (hit_row_of(state, &frame, now), area) {
                (Some(row), Some(area)) => OutputFrame {
                    windows: row.touching(&frame.windows, area),
                    ..frame
                },
                _ => frame,
            }
        })
        .collect();
    FrameScene { outputs }
}

/// An output's frame with nothing on it (no window moves there).
pub(crate) fn empty_frame(output: &OutputKey) -> OutputFrame {
    OutputFrame {
        output: output.clone(),
        strip: StripOffset(SubPx(0)),
        windows: Vec::new(),
    }
}

/// The windows that are full screen: drawn and hit above the top layers.
fn full_screen_windows(model: &Model) -> Vec<WindowId> {
    model
        .windows()
        .iter()
        .filter(|(_, window)| matches!(window.state, WindowState::FullScreen { .. }))
        .map(|(id, _)| *id)
        .collect()
}

/// The layer surfaces on `output`, by level, bottom of each level first (key order), where the
/// model arranges them, and the full-screen windows.
pub(crate) fn layers_on(model: &Model, output: &OutputKey) -> LayerDraw {
    let covering = LayerDraw {
        full_screen: full_screen_windows(model),
        arranged: model
            .outputs()
            .0
            .get(output)
            .map(|state| model.layers().arrange(output, state.config.logical_rect())),
        ..LayerDraw::default()
    };
    model
        .layers()
        .iter()
        .filter(|(_, spec)| spec.output == *output)
        .fold(covering, |mut draw, (key, spec)| {
            let level = match spec.level {
                LayerLevel::Background => &mut draw.background,
                LayerLevel::Bottom => &mut draw.bottom,
                LayerLevel::Top => &mut draw.top,
                LayerLevel::Overlay => &mut draw.overlay,
            };
            level.push(*key);
            draw.blur_strengths
                .insert(*key, BlurStrength::of(spec.role));
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

impl FrameElements for SceneSource<'_> {
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

/// Every drawn surface that takes input, front to back: lock surfaces, overlay layers, the bar and
/// dock an edge reveal raised, full-screen windows, top layers, the other windows by stacking (the
/// sampled frame's, top first), bottom then background layers; each root is preceded by its
/// popups. Each with where its top-left is in global logical pixels.
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
                    clip: None,
                },
            ))
        });
    let arranged: BTreeMap<OutputKey, Arranged> = model
        .outputs()
        .0
        .iter()
        .map(|(output, state)| {
            let full = state.config.logical_rect();
            (output.clone(), model.layers().arrange(output, full))
        })
        .collect();
    // The bar and the dock an edge reveal has raised: over the full-screen window, where they
    // are drawn now.
    let raised = super::reveal::raised(state, state.clock.now());
    let raised_hits: Vec<(Hit, Candidate)> = raised
        .iter()
        .rev()
        .filter_map(|draw| {
            let entry = book.layer(&draw.layer)?;
            Some((
                Hit::Layer(draw.layer),
                Candidate {
                    root: entry.root.clone(),
                    origin: draw.area.origin,
                    clip: Some(draw.area),
                },
            ))
        })
        .collect();
    let layer = |levels: &'static [LayerLevel]| {
        let mut found: Vec<(Hit, Candidate)> = model
            .layers()
            .iter()
            .filter(|(_, spec)| levels.contains(&spec.level))
            .filter(|(key, _)| !raised.iter().any(|draw| draw.layer == **key))
            .filter_map(|(key, spec)| {
                let entry = book.layer(key)?;
                let area = arranged.get(&spec.output)?.place(key, entry.committed)?;
                Some((
                    Hit::Layer(*key),
                    Candidate {
                        root: entry.root.clone(),
                        origin: area.origin,
                        clip: Some(area),
                    },
                ))
            })
            .collect();
        found.reverse();
        found
    };
    let windows = drawn_scene_at(state, state.clock.now())
        .outputs
        .into_iter()
        .flat_map(|frame| frame.windows.into_iter().rev())
        // A window in a minimise flight is drawn shrunk or bent, so its real rectangle is not
        // where it is: it takes no pointer input until it has landed.
        .filter(|placed| placed.flight.is_none())
        .filter_map(|placed| {
            let entry = book.window(&placed.window)?;
            Some((
                Hit::Window(placed.window),
                Candidate {
                    root: entry.root.clone(),
                    origin: surface_origin_logical(
                        &entry.root,
                        LogicalPoint {
                            x: placed.at.x.round_logical(),
                            y: placed.at.y.round_logical(),
                        },
                    ),
                    clip: None,
                },
            ))
        });
    let full_screen = full_screen_windows(model);
    let (covering, others): (Vec<_>, Vec<_>) = windows
        .into_iter()
        .partition(|(hit, _)| matches!(hit, Hit::Window(id) if full_screen.contains(id)));
    locks
        .chain(layer(&[LayerLevel::Overlay]))
        .chain(raised_hits)
        .chain(covering)
        .chain(layer(&[LayerLevel::Top]))
        .chain(others)
        .chain(layer(&[LayerLevel::Bottom]))
        .chain(layer(&[LayerLevel::Background]))
        .flat_map(with_popups)
        .collect()
}

/// A root surface and its popups, popups first (a menu hangs over what it came from), each
/// belonging to the same `Hit` and placed where the renderer draws it.
fn with_popups((hit, root): (Hit, Candidate)) -> Vec<(Hit, Candidate)> {
    let mut found: Vec<(Hit, Candidate)> = popup_surfaces(&root.root, root.origin)
        .into_iter()
        .map(|(surface, origin)| {
            (
                hit.clone(),
                Candidate {
                    root: surface,
                    origin,
                    clip: None,
                },
            )
        })
        .collect();
    found.push((hit, root));
    found
}

/// The first surface in `stack` with input at `at`, and what it belongs to.
pub(super) fn hit_test(stack: &[(Hit, Candidate)], at: LogicalPoint) -> Option<(Hit, SurfaceAt)> {
    stack.iter().find_map(|(hit, candidate)| {
        surface_under(std::slice::from_ref(candidate), at).map(|found| (hit.clone(), found))
    })
}
