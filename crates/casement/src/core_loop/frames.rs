//! The server-side frame in the loop: which windows have one and how it looks, what the pointer
//! is over on a frame, and what a press on it does (a light, a drag on the titlebar, a resize
//! edge). The drawing is `casement-render`'s and the decisions are `casement-core`'s
//! (`window::frame`, `window::frame_press`); this is the glue between them and the seat.

use super::deliver::{mac_mods, next_serial, run_model};
use super::feed;
use super::scene::{Hit, hit_test, stack};
use crate::exec;
use crate::state::State;
use casement_core::focus::{HitPart, Press};
use casement_core::geometry::{LogicalPoint, LogicalRect};
use casement_core::input::{ButtonCode, InputEvent};
use casement_core::window::{ChromeAction, FrameHit, FrameStyle, WindowState, chrome_action};
use casement_core::{Effect, Event, ResizeEdges, ToplevelRequest, WindowId};
use casement_layout::{GroupHover, Scheme, frame_style};
use casement_protocols::vocab::Emphasis;
use casement_render::{CursorDraw, CursorShape, window_content};
use casement_wayland::seat::{Candidate, SurfaceAt, surface_under};
use casement_wayland::serial::{PressKind, PressRecord};
use casement_wayland::xdg_decoration::DecorationMode;
use ds_core::vocab::Activity;
use smithay::input::Seat;
use std::collections::BTreeMap;

/// What the pointer is on, when it is on a window's frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Over {
    /// The window.
    pub(crate) window: WindowId,
    /// The part: a light, the titlebar or a resize zone.
    pub(crate) hit: FrameHit,
}

/// The windows whose frame casement draws and how each looks now: the ones whose client was
/// answered server-side, and that are neither full screen nor in the dock.
pub(super) fn styles(state: &State) -> BTreeMap<WindowId, FrameStyle> {
    let over = state.render.frame_over;
    state
        .core
        .model
        .windows()
        .iter()
        .filter(|(id, _)| state.wl.xdg_decoration.mode_of(**id) == DecorationMode::ServerSide)
        .filter(|(_, window)| {
            !matches!(
                window.state,
                WindowState::FullScreen { .. } | WindowState::Minimised { .. }
            )
        })
        .map(|(id, window)| {
            let activity = match window.emphasis {
                Emphasis::Key | Emphasis::Main => Activity::Active,
                _ => Activity::Inactive,
            };
            let hover = match over {
                Some(Over {
                    window: hovered,
                    hit: FrameHit::Light(_),
                }) if hovered == *id => GroupHover::Over,
                _ => GroupHover::Away,
            };
            (*id, frame_style(Scheme::Light, activity, hover))
        })
        .collect()
}

/// What is under `at`: a frame part, or else the surface there.
pub(super) fn pick(state: &State, at: LogicalPoint) -> (Option<Over>, Option<(Hit, SurfaceAt)>) {
    let stacked = stack(state);
    let over = under(state, &stacked, at);
    let surface = match over {
        Some(_) => None,
        None => hit_test(&stacked, at),
    };
    (over, surface)
}

/// The frame part under `at`, when a frame (and not a surface in front of it) is what the
/// pointer is on. `stacked` is the scene's surfaces front to back; each window's frame sits
/// at its root's place in that order, so a window in front hides the frame behind it.
pub(super) fn under(state: &State, stacked: &[(Hit, Candidate)], at: LogicalPoint) -> Option<Over> {
    let styles = styles(state);
    if styles.is_empty() {
        return None;
    }
    let contents = contents(state);
    for (hit, candidate) in stacked {
        if let Hit::Window(window) = hit
            && let (Some(style), Some(content)) = (styles.get(window), contents.get(window))
            && state
                .render
                .textures
                .window(window)
                .is_some_and(|entry| entry.root == candidate.root)
        {
            match style.around(*content).hit(at) {
                FrameHit::Outside | FrameHit::Content => {}
                found => {
                    return Some(Over {
                        window: *window,
                        hit: found,
                    });
                }
            }
        }
        if surface_under(std::slice::from_ref(candidate), at).is_some() {
            return None;
        }
    }
    None
}

/// Each window's content rectangle as it is drawn now.
fn contents(state: &State) -> BTreeMap<WindowId, LogicalRect> {
    let book = &state.render.textures;
    super::scene::sample_scene_at(state, state.clock.now())
        .outputs
        .into_iter()
        .flat_map(|frame| frame.windows)
        .filter_map(|placed| {
            let entry = book.window(&placed.window)?;
            Some((placed.window, window_content(&placed, &entry.root)))
        })
        .collect()
}

/// The pointer moved: what it is over on a frame is remembered (the lights show their marks
/// while it is over them) and the cursor follows the part. Leaving a frame puts the arrow back
/// until the client under the pointer asks for its own.
pub(super) fn hover(state: &mut State, now_over: Option<Over>) {
    let before = std::mem::replace(&mut state.render.frame_over, now_over);
    match (before, now_over) {
        (_, Some(Over { hit, .. })) => state.render.chrome.cursor = CursorDraw::Named(shape(hit)),
        (Some(_), None) => state.render.chrome.cursor = CursorDraw::Named(CursorShape::Default),
        (None, None) => {}
    }
}

/// The cursor over a frame part.
fn shape(hit: FrameHit) -> CursorShape {
    match hit {
        FrameHit::Edge(ResizeEdges::Top | ResizeEdges::Bottom) => CursorShape::ResizeNs,
        FrameHit::Edge(ResizeEdges::Left | ResizeEdges::Right) => CursorShape::ResizeEw,
        FrameHit::Edge(ResizeEdges::TopLeft | ResizeEdges::BottomRight) => CursorShape::ResizeNwse,
        FrameHit::Edge(ResizeEdges::TopRight | ResizeEdges::BottomLeft) => CursorShape::ResizeNesw,
        _ => CursorShape::Default,
    }
}

/// `button` went down on a frame part: the model routes the press (it activates and raises the
/// window as it would for a press on its content), then the part asks for its action. The
/// press is never delivered to a client; its release is swallowed with it.
pub(super) fn press(
    state: &mut State,
    seat: &Seat<State>,
    event: &InputEvent,
    button: ButtonCode,
    over: Over,
) {
    let window = over.window;
    let mods = seat
        .get_keyboard()
        .map_or(casement_protocols::vocab::MacMods::NONE, |keyboard| {
            mac_mods(&keyboard.modifier_state())
        });
    let part = match over.hit {
        FrameHit::Edge(_) => HitPart::Edge,
        _ => HitPart::Titlebar,
    };
    let press = Press {
        window,
        part,
        mods,
        origin: event.origin,
    };
    let effects = run_model(state, |model, now| model.press(press, now));
    exec::exec(state, effects);
    let at = pointer_point(seat);
    let (tracker, clicks) = state.render.clicks.press(window, event.at, at);
    state.render.clicks = match over.hit {
        FrameHit::Titlebar => tracker,
        _ => state.render.clicks,
    };
    let Some(held) = state.core.model.windows().get(&window) else {
        return;
    };
    let double = state.core.model.settings().windows.double_click_title;
    let client = held.client;
    let Some(action) = chrome_action(over.hit, clicks, double, &held.state) else {
        return;
    };
    let serial = next_serial();
    state.wl.seat.record_press(
        serial,
        PressRecord {
            client,
            at: event.at,
            kind: PressKind::Button,
            origin: event.origin,
            button: Some(button),
        },
    );
    carry_out(state, window, action, serial);
}

/// Where the pointer is, in whole logical pixels.
fn pointer_point(seat: &Seat<State>) -> LogicalPoint {
    seat.get_pointer()
        .map_or(LogicalPoint { x: 0, y: 0 }, |pointer| {
            let at = pointer.current_location();
            LogicalPoint {
                x: at.x.floor() as i32,
                y: at.y.floor() as i32,
            }
        })
}

/// The window does what its frame asked.
fn carry_out(
    state: &mut State,
    window: WindowId,
    action: ChromeAction,
    serial: casement_core::InputSerial,
) {
    let request = match action {
        ChromeAction::Close => {
            exec::exec(state, vec![Effect::SendClose { window }]);
            return;
        }
        ChromeAction::Minimise => ToplevelRequest::Minimise,
        ChromeAction::Maximise(toggle) => ToplevelRequest::Maximise(toggle),
        ChromeAction::Move => ToplevelRequest::Move { serial },
        ChromeAction::Resize(edges) => ToplevelRequest::Resize { serial, edges },
        _ => return,
    };
    feed(state, vec![Event::ToplevelRequest { window, request }]);
}
