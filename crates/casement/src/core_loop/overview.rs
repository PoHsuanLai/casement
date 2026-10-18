//! Mission Control in the loop: while it is not closed the windows take no pointer input (they
//! are drawn scaled in their slots, not where they sit), a press picks the window under it or
//! leaves, hovering rings the window and shows its title, and Escape leaves
//! (`deliver::deliver_key`). The slots, the flight and the open/leave states are layout's,
//! motion's and the model's; this is the glue between them and the seat.

use super::deliver::run_model;
use super::fingers::openness;
use super::frames;
use super::overview_drag as drag;
use crate::exec;
use crate::icons::book::want_of;
use crate::state::{RenderState, State};
use casement_chrome::{Margins, TitleKey, TitleRaster};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::input::InputEvent;
use casement_core::overview::Overview;
use casement_core::overview_drag::Drag;
use casement_core::raster::RasterImage;
use casement_core::{OverviewPick, WindowId};
use casement_layout::{Scheme, overview_dim, overview_plate, overview_ring};
use casement_render::{
    CursorDraw, CursorShape, OutputView, OverviewDraw, OverviewHover, OverviewIcon, OverviewLook,
    Rgba8, shown_length,
};
use casement_wayland::xdg_toplevel_icon::icon_of;
use ds_core::vocab::Activity;
use std::collections::BTreeMap;

/// The narrowest a window's title under it is made (a very small window still gets a readable
/// label), in logical pixels.
const LABEL_MIN: u32 = 160;

/// The widest a title under a window is made.
const LABEL_MAX: u32 = 480;

/// The height of the title under a window.
const LABEL_HEIGHT: u32 = 22;

/// Whether Mission Control has the pointer: from the moment it opens until its windows are back
/// (the windows take no input while they fly in or out).
pub(super) fn takes_input(state: &State) -> bool {
    !matches!(state.core.model.overview(), Overview::Closed)
}

/// Whether Escape leaves now: it is entering or open (leaving already is).
pub(super) fn escape_leaves(state: &State) -> bool {
    matches!(
        state.core.model.overview(),
        Overview::Entering | Overview::Open
    )
}

/// The window whose slot, with its frame above it, holds `at`; the front-most one. The frame is
/// shrunk with the window, so the part above the slot is the titlebar's height at the slot's
/// scale (the window's size on the desktop is the target the layout kept for it).
pub(super) fn window_at(state: &State, at: LogicalPoint) -> Option<WindowId> {
    let reserves = frames::reserves(state);
    let arrangement = &state.core.arrangement;
    arrangement
        .overview
        .iter()
        .rev()
        .find(|slot| {
            let full = arrangement
                .windows
                .iter()
                .find(|target| target.window == slot.window)
                .map_or(slot.rect.size, |target| target.rect.size);
            let bar = reserves.get(&slot.window).copied().unwrap_or(0);
            reach(slot.rect, shown_length(bar, slot.rect.size, full)).contains(at)
        })
        .map(|slot| slot.window)
}

/// `slot` with `bar` logical pixels of frame above it.
fn reach(slot: LogicalRect, bar: u32) -> LogicalRect {
    LogicalRect::new(
        LogicalPoint {
            x: slot.origin.x,
            y: slot
                .origin
                .y
                .saturating_sub(i32::try_from(bar).unwrap_or(0)),
        },
        LogicalSize {
            w: slot.size.w,
            h: slot.size.h.saturating_add(bar),
        },
    )
}

/// The pointer is at `at` while Mission Control has it: the window there is the hovered one
/// (only once it is entering or open), the arrow is the cursor, and the frame hover the desktop
/// had is gone.
pub(super) fn hover(state: &mut State, at: LogicalPoint) {
    frames::hover(state, None);
    state.render.chrome.cursor = CursorDraw::Named(CursorShape::Default);
    if drag::moved(state, at) == drag::Follow::Taken {
        return;
    }
    state.render.overview_hover = match escape_leaves(state) {
        true => window_at(state, at),
        false => None,
    };
}

/// A press at `at`: the window under it is picked and Mission Control leaves with it raised and
/// keyed; a press on the picture leaves with nothing picked. While it is already leaving the
/// press does nothing.
pub(super) fn press(state: &mut State, event: &InputEvent, at: LogicalPoint) {
    if !escape_leaves(state) {
        return;
    }
    // A press on a slot waits for its release (a pick) or its move past the threshold (a drag
    // to a Space); a press on the picture leaves at once.
    match window_at(state, at) {
        Some(window) => drag::press(state, event, window, at),
        None => leave(state, None),
    }
}

/// The button went up: the press on a slot ends (`overview_drag`).
pub(super) fn released(state: &mut State) {
    drag::released(state);
}

/// Mission Control leaves, with `pick` raised and keyed when there is one.
pub(super) fn leave(state: &mut State, pick: Option<OverviewPick>) {
    state.render.overview_hover = None;
    drag::abandon(state);
    let effects = run_model(state, |model, now| model.close_overview(pick, now));
    exec::exec(state, effects);
}

/// How Mission Control is drawn this frame: not at all while closed, else with the hovered
/// window's title under it. While the fingers hold it part-way it is drawn too (the windows are
/// scaled to where the fingers put them), dimmed as far as it is open, with no ring, label or
/// icons.
pub(super) fn draw(state: &mut State, view: &OutputView) -> OverviewDraw {
    let fingers = state.core.model.overview_progress();
    if !takes_input(state) && fingers.is_none() {
        state.render.overview_hover = None;
        state.render.icons.clear();
        return OverviewDraw::Off;
    }
    let hover = state
        .render
        .overview_hover
        .filter(|_| escape_leaves(state))
        .map(|window| OverviewHover {
            window,
            label: label_of(state, window, view),
        });
    let scheme = state.render.scheme;
    let look = OverviewLook::new(
        Rgba8::from(overview_dim(scheme)),
        Rgba8::from(overview_ring(scheme)),
    );
    let look = match fingers {
        Some(progress) => look.opened(openness(progress)),
        None => look,
    };
    // The window being dragged is drawn above the others.
    let lift = match &state.render.overview_drag {
        Drag::Dragging { window, .. } => Some(*window),
        _ => None,
    };
    let icons = match takes_input(state) {
        true => icons_of(state, view),
        false => BTreeMap::new(),
    };
    OverviewDraw::On {
        hover,
        look,
        lift,
        icons,
    }
}

/// The icon under each window of Mission Control whose icon is on this output: found the first
/// frame Mission Control is up (this reads files), kept until it is closed. On the way out the
/// layout has no slots any more, so the icons last drawn leave with their windows.
fn icons_of(state: &mut State, view: &OutputView) -> BTreeMap<WindowId, OverviewIcon> {
    let plate = overview_plate(state.render.scheme);
    let ink = [plate.r, plate.g, plate.b, plate.a];
    let RenderState {
        icons, textures, ..
    } = &mut state.render;
    let model = &state.core.model;
    let slots = &state.core.arrangement.overview;
    let here = |rect: &LogicalRect| view.area.intersection(rect).is_some();
    if slots.is_empty() {
        return icons
            .remembered()
            .iter()
            .filter(|(_, icon)| here(&icon.rest))
            .map(|(window, icon)| (*window, icon.clone()))
            .collect();
    }
    let found: BTreeMap<WindowId, OverviewIcon> = slots
        .iter()
        .filter(|slot| here(&slot.icon))
        .filter_map(|slot| {
            let window = model.windows().get(&slot.window)?;
            let draw = icons.draw_of(slot.window, slot.icon, view.scale, ink, || {
                let set = textures
                    .window(&slot.window)
                    .and_then(|entry| icon_of(&entry.root));
                want_of(window.claimed_id.as_str(), set)
            })?;
            let icon = OverviewIcon {
                rest: slot.icon,
                slot: slot.rect,
                draw,
            };
            Some((slot.window, icon))
        })
        .collect();
    icons.remember(&found);
    found
}

/// `window`'s title as a label under its slot: as wide as the slot, within bounds.
fn label_of(
    state: &mut State,
    window: WindowId,
    view: &OutputView,
) -> Option<(LogicalSize, RasterImage)> {
    let title = &state.core.model.windows().get(&window)?.title.0;
    let slot = state
        .core
        .arrangement
        .overview
        .iter()
        .find(|slot| slot.window == window)?;
    let size = LogicalSize {
        w: slot.rect.size.w.clamp(LABEL_MIN, LABEL_MAX),
        h: LABEL_HEIGHT,
    };
    let raster = state
        .render
        .labels
        .get_or_insert_with(|| TitleRaster::system(Margins { left: 0, right: 0 }));
    let key = TitleKey::new(
        title,
        size.w,
        size.h,
        view.scale,
        Scheme::Dark,
        Activity::Active,
    );
    let draw = raster.render(&key).ok()?;
    Some((size, draw))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    #[test]
    fn a_slots_reach_includes_the_frame_above_it() {
        let slot = rect(100, 200, 300, 150);
        // name, bar, point, inside
        let cases = [
            ("the content", 28, (150, 250), true),
            ("the frame", 28, (150, 180), true),
            ("above the frame", 28, (150, 170), false),
            ("no frame, where it would be", 0, (150, 180), false),
            ("below", 28, (150, 350), false),
            ("right of it", 28, (400, 250), false),
        ];
        for (name, bar, (x, y), inside) in cases {
            assert_eq!(
                reach(slot, bar).contains(LogicalPoint { x, y }),
                inside,
                "{name}"
            );
        }
    }
}
