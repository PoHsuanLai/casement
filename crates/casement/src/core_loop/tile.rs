//! Dragging a window to a screen edge, as the loop carries it: a move grab starts the machine
//! (`casement_core::window::TileDrag`), each pointer motion steps it with the modifiers held,
//! the frame draws its preview, and the release tiles the window where the preview showed. The
//! machine and the zones are `casement-core`'s and `casement-layout`'s; this is the glue to the
//! seat, the model and the renderer.
//!
//! A window tiles on its own output only: the pointer on another output arms nothing (a window
//! keeps its Space's output while it is dragged).

use super::deliver::held_mods;
use super::feed;
use crate::state::State;
use casement_core::geometry::{LogicalPoint, LogicalRect};
use casement_core::window::{TileDrag, TileDrop, TileKnobs, TileScene, TileTarget};
use casement_core::{Event, GrabSpec, InputSerial, OutputKey, ToplevelRequest};
use casement_layout::{LayoutKnobs, tile_preview_fill, zone_rect};
use casement_protocols::vocab::HeldMods;
use casement_render::{Rgba8, TilePreview};

/// The plate's corner radius, in logical pixels (a window's corner on the Mac).
const PLATE_RADIUS: u32 = 10;

/// A grab of `spec` begins: a move may tile, a resize never does.
pub(super) fn begin(state: &mut State, spec: GrabSpec) {
    state.render.tile_drag = match spec {
        GrabSpec::Move { window, .. } => TileDrag::begin(window),
        GrabSpec::Resize { .. } => TileDrag::Idle,
    };
}

/// The pointer is at `at`: the zone it arms follows.
pub(super) fn moved(state: &mut State, at: LogicalPoint) {
    let drag = std::mem::take(&mut state.render.tile_drag);
    let Some(window) = drag.window() else {
        return;
    };
    let mods = state
        .wl
        .seat
        .seat
        .get_keyboard()
        .map_or(HeldMods::NONE, |keyboard| {
            held_mods(&keyboard.modifier_state())
        });
    let model = &state.core.model;
    let knobs = TileKnobs::of(&model.settings().windows);
    let rects = model.output_rects();
    let home = model
        .windows()
        .get(&window)
        .and_then(|held| model.output_of(held));
    let next = match rects.iter().find(|(key, _)| Some(key) == home.as_ref()) {
        Some((home_key, home_rect)) => {
            let others: Vec<LogicalRect> = rects
                .iter()
                .filter(|(key, _)| key != home_key)
                .map(|(_, rect)| *rect)
                .collect();
            let scene = TileScene {
                home: *home_rect,
                others: &others,
            };
            drag.moved(at, mods, &scene, &knobs)
        }
        None => TileDrag::begin(window),
    };
    state.render.tile_drag = next;
}

/// The move ended with the button of the press `serial`: the window tiles when a zone was armed.
pub(super) fn released(state: &mut State, serial: InputSerial) {
    let (next, drop) = std::mem::take(&mut state.render.tile_drag).released();
    state.render.tile_drag = next;
    if let Some(TileDrop { window, target }) = drop {
        let request = ToplevelRequest::Place {
            target: target.place(),
            serial,
        };
        feed(state, vec![Event::ToplevelRequest { window, request }]);
    }
}

/// The move ended some other way (the window is gone): the drag is over, nothing tiles.
pub(super) fn cancel(state: &mut State) {
    state.render.tile_drag = TileDrag::Idle;
}

/// The plate to draw on `output`: the armed zone of the window dragged on it.
pub(super) fn preview(state: &State, output: &OutputKey) -> Option<TilePreview> {
    let drag = &state.render.tile_drag;
    let (window, target) = (drag.window()?, drag.target()?);
    let model = &state.core.model;
    let held = model.windows().get(&window)?;
    if model.output_of(held)? != *output {
        return None;
    }
    let full = model
        .output_rects()
        .into_iter()
        .find(|(key, _)| key == output)
        .map(|(_, rect)| rect)?;
    let usable = model.layers().usable_area(output, full);
    let margin = LayoutKnobs::of(model.settings()).tile_margin;
    let rect = match target {
        TileTarget::Zone(zone) => zone_rect(zone, usable, margin),
        TileTarget::Fill => usable,
    };
    Some(TilePreview {
        window,
        rect,
        fill: Rgba8::from(tile_preview_fill(state.render.scheme)),
        radius: PLATE_RADIUS,
    })
}
