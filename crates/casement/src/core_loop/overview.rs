//! Mission Control in the loop: while it is not closed the windows take no pointer input (they
//! are drawn scaled in their slots, not where they sit), a press picks the window under it or
//! leaves, hovering rings the window and shows its title, and Escape leaves
//! (`deliver::deliver_key`). The slots, the flight and the open/leave states are layout's,
//! motion's and the model's; this is the glue between them and the seat.

use super::deliver::run_model;
use super::frames;
use crate::exec;
use crate::state::State;
use casement_chrome::{Margins, TitleKey, TitleRaster};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::input::InputEvent;
use casement_core::overview::Overview;
use casement_core::{OverviewPick, WindowId};
use casement_layout::Scheme;
use casement_render::{
    CursorDraw, CursorShape, OutputView, OverviewDraw, OverviewHover, TitleDraw,
};
use ds_core::vocab::Activity;

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

/// The window whose slot, with its frame above it, holds `at`; the front-most one.
pub(super) fn window_at(state: &State, at: LogicalPoint) -> Option<WindowId> {
    let reserves = frames::reserves(state);
    state
        .core
        .arrangement
        .overview
        .iter()
        .rev()
        .find(|slot| {
            let bar = reserves.get(&slot.window).copied().unwrap_or(0);
            reach(slot.rect, bar).contains(at)
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
    let pick = window_at(state, at).map(|window| OverviewPick {
        window,
        origin: event.origin,
    });
    leave(state, pick);
}

/// Mission Control leaves, with `pick` raised and keyed when there is one.
pub(super) fn leave(state: &mut State, pick: Option<OverviewPick>) {
    state.render.overview_hover = None;
    let effects = run_model(state, |model, now| model.close_overview(pick, now));
    exec::exec(state, effects);
}

/// How Mission Control is drawn this frame: not at all while closed, else with the hovered
/// window's title under it.
pub(super) fn draw(state: &mut State, view: &OutputView) -> OverviewDraw {
    if !takes_input(state) {
        state.render.overview_hover = None;
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
    OverviewDraw::On { hover }
}

/// `window`'s title as a label under its slot: as wide as the slot, within bounds.
fn label_of(
    state: &mut State,
    window: WindowId,
    view: &OutputView,
) -> Option<(LogicalSize, TitleDraw)> {
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
    let pixels = raster.render(&key).ok()?;
    let draw = TitleDraw {
        id: pixels.id,
        width: pixels.width,
        height: pixels.height,
        rgba: pixels.rgba,
    };
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
