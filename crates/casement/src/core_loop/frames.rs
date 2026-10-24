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
use casement_core::window::{
    ArmInput, ArmOutcome, Cast, ChromeAction, Clicks, FrameHit, FrameLook, FrameStyle, LightArm,
    LightAt, WindowState, chrome_action, round_of,
};
use casement_core::{Effect, Event, ResizeEdges, ToplevelRequest, WindowId};
use casement_layout::{GroupHover, frame_metrics, frame_style, pressed};
use casement_protocols::vocab::{Emphasis, MacMods, WindowRole};
use casement_render::{
    ClientRound, ClientShadow, CursorDraw, CursorShape, margin_of, window_content,
};
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

/// The windows that have a server-side frame to act on now (hit, reserve): the ones whose
/// client was answered server-side, and that are neither full screen nor in the dock.
pub(super) fn styles(state: &State) -> BTreeMap<WindowId, FrameStyle> {
    styles_where(state, framed)
}

/// The frames to draw this frame: [`styles`], and the frame of a window on its way into the
/// dock, which flies and fades with its content. A window the dock holds is drawn only while
/// the frame scene still has it (it leaves the scene when its fade ends), so a frame is never
/// drawn for a window nothing else draws. The Mission Control and Space-slide passes draw
/// through the same `output_elements` call, so a window they place has its frame, shadow and
/// rounded clip at the place it was given.
pub(super) fn drawn(state: &State) -> BTreeMap<WindowId, FrameStyle> {
    styles_where(state, drawn_state)
}

/// The windows that draw their own frame and that casement shapes: the corners clipped at the
/// radius `round_of` gives, and the shadow of a framed window's activity where it casts one. Not
/// the windows casement frames, not a full-screen window (square, as a framed one is), not the
/// desktop's own shell windows; a window in the dock stays so its genie is cut the same.
pub(super) fn rounds(state: &State) -> BTreeMap<WindowId, ClientRound> {
    let book = &state.render.textures;
    state
        .core
        .model
        .windows()
        .iter()
        .filter(|(id, _)| state.wl.xdg_decoration.mode_of(**id) != DecorationMode::ServerSide)
        .filter(|(_, window)| {
            !matches!(window.state, WindowState::FullScreen { .. })
                && window.role != WindowRole::Shell
        })
        .filter_map(|(id, window)| {
            let entry = book.window(id)?;
            let style = frame_style(state.render.scheme, activity_of(window), GroupHover::Away);
            let round = round_of(
                window.hints.frame_choice(),
                margin_of(&entry.root),
                style.look.radius,
            )?;
            let shadow = match round.shadow {
                Cast::Casement => ClientShadow::Cast(FrameStyle {
                    look: FrameLook {
                        radius: round.radius,
                        ..style.look
                    },
                    ..style
                }),
                Cast::Client => ClientShadow::Client,
            };
            let radius = round.radius;
            Some((*id, ClientRound { radius, shadow }))
        })
        .collect()
}

/// Whether a window in `state` has its frame drawn: [`framed`], or on its way into the dock.
fn drawn_state(state: &WindowState) -> bool {
    framed(state) || matches!(state, WindowState::Minimised { .. })
}

/// How each decorated window, whose state `keep` accepts, looks now.
fn styles_where(state: &State, keep: fn(&WindowState) -> bool) -> BTreeMap<WindowId, FrameStyle> {
    let over = state.render.frame_over;
    let down = state.render.light_arm.pressed();
    decorated(state, keep)
        .map(|(id, window)| {
            let activity = activity_of(window);
            let hover = match over {
                Some(Over {
                    window: hovered,
                    hit: FrameHit::Light(_),
                }) if hovered == *id => GroupHover::Over,
                _ => GroupHover::Away,
            };
            let style = frame_style(state.render.scheme, activity, hover);
            let style = match down {
                Some(LightAt { window, light }) if window == *id => {
                    pressed(style, light, state.render.scheme)
                }
                _ => style,
            };
            (*id, style)
        })
        .collect()
}

/// Whether `window` is the one the person works in: its frame and title draw in the key ink.
pub(super) fn activity_of(window: &casement_core::window::Window) -> Activity {
    match window.emphasis {
        Emphasis::Key | Emphasis::Main => Activity::Active,
        _ => Activity::Inactive,
    }
}

/// The windows whose client was answered server-side and whose state `keep` accepts.
fn decorated(
    state: &State,
    keep: fn(&WindowState) -> bool,
) -> impl Iterator<Item = (&WindowId, &casement_core::window::Window)> {
    state
        .core
        .model
        .windows()
        .iter()
        .filter(|(id, _)| state.wl.xdg_decoration.mode_of(**id) == DecorationMode::ServerSide)
        .filter(move |(_, window)| keep(&window.state))
}

/// Whether a window in `state` has a server-side frame, and so its content is clipped at the
/// bottom corners (`casement-render`'s rounded clip): every state but full screen and the
/// dock. A filled (zoomed) window keeps its radius, as on the Mac; full screen drops it.
fn framed(state: &WindowState) -> bool {
    !matches!(
        state,
        WindowState::FullScreen { .. } | WindowState::Minimised { .. }
    )
}

/// How far each framed window's frame reaches above its content (the titlebar's height), for
/// layout: fill, first placement and the keep-inside clamp hold the frame in the usable area.
pub(super) fn reserves(state: &State) -> BTreeMap<WindowId, u32> {
    let bar = frame_metrics().titlebar;
    decorated(state, framed).map(|(id, _)| (*id, bar)).collect()
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
pub(super) fn contents(state: &State) -> BTreeMap<WindowId, LogicalRect> {
    let book = &state.render.textures;
    super::scene::drawn_scene_at(state, state.clock.now())
        .outputs
        .into_iter()
        .flat_map(|frame| frame.windows)
        // A window in a minimise flight takes no pointer input (see `scene::stack`).
        .filter(|placed| placed.flight.is_none())
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
    let (arm, _) = state
        .render
        .light_arm
        .step(ArmInput::Move(light_of(now_over)));
    state.render.light_arm = arm;
    super::menu::dwell(state, light_of(now_over));
    let before = std::mem::replace(&mut state.render.frame_over, now_over);
    match (before, now_over) {
        (_, Some(Over { hit, .. })) => state.render.chrome.cursor = CursorDraw::Named(shape(hit)),
        (Some(_), None) => state.render.chrome.cursor = CursorDraw::Named(CursorShape::Default),
        (None, None) => {}
    }
}

/// The light `over` is, when it is one.
pub(super) fn light_of(over: Option<Over>) -> Option<LightAt> {
    match over {
        Some(Over {
            window,
            hit: FrameHit::Light(light),
        }) => Some(LightAt { window, light }),
        _ => None,
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
    let mods = mods_of(seat);
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
    if let FrameHit::Light(light) = over.hit {
        // A light acts when the button comes up on it (`released`); now it only shows pressed.
        let on = LightAt { window, light };
        let (arm, _) = state.render.light_arm.step(ArmInput::Press { on, button });
        state.render.light_arm = arm;
        super::menu::pressed(state);
        return;
    }
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
    let Some(action) = chrome_action(over.hit, clicks, double, mods, &held.state) else {
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

/// `button` came up: a light held down fires when the pointer is still on it, and is let go
/// otherwise. The release itself was swallowed with its press.
pub(super) fn released(state: &mut State, seat: &Seat<State>, button: ButtonCode) {
    if state.render.light_arm == LightArm::Idle {
        return;
    }
    let (over, _) = pick(state, pointer_point(seat));
    let (arm, outcome) = state.render.light_arm.step(ArmInput::Release {
        button,
        over: light_of(over),
    });
    state.render.light_arm = arm;
    if let ArmOutcome::Fire(on) = outcome {
        fire(state, seat, on);
    }
}

/// The light `on` was clicked: the window does what it does under the modifiers held now.
fn fire(state: &mut State, seat: &Seat<State>, on: LightAt) {
    let Some(held) = state.core.model.windows().get(&on.window) else {
        return;
    };
    let double = state.core.model.settings().windows.double_click_title;
    let Some(action) = chrome_action(
        FrameHit::Light(on.light),
        Clicks::Single,
        double,
        mods_of(seat),
        &held.state,
    ) else {
        return;
    };
    carry_out(state, on.window, action, next_serial());
}

/// The Mac modifiers held now.
fn mods_of(seat: &Seat<State>) -> MacMods {
    seat.get_keyboard().map_or(MacMods::NONE, |keyboard| {
        mac_mods(&keyboard.modifier_state())
    })
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
        ChromeAction::FullScreen(toggle) => ToplevelRequest::FullScreen {
            toggle,
            output: None,
        },
        ChromeAction::Move => ToplevelRequest::Move { serial },
        ChromeAction::Resize(edges) => ToplevelRequest::Resize { serial, edges },
        _ => return,
    };
    feed(state, vec![Event::ToplevelRequest { window, request }]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalPoint, LogicalSize};
    use casement_core::window::Restore;
    use casement_protocols::vocab::TileZone;
    use porter_core::SpaceId;

    #[test]
    fn which_states_have_a_frame_and_so_rounded_corners() {
        let rect = || LogicalRect::new(LogicalPoint { x: 1, y: 2 }, LogicalSize { w: 30, h: 40 });
        let space = SpaceId::parse("s-1").expect("id");
        let table = [
            (
                "unplaced",
                WindowState::New(LogicalSize { w: 30, h: 40 }),
                true,
            ),
            ("floating", WindowState::Floating(rect()), true),
            (
                "tiled",
                WindowState::Tiled {
                    zone: TileZone::TopRight,
                    restore: rect(),
                },
                true,
            ),
            ("zoomed", WindowState::Filled { restore: rect() }, true),
            (
                "full screen",
                WindowState::FullScreen {
                    space,
                    restore: Restore::Floating(rect()),
                },
                false,
            ),
            (
                "in the dock",
                WindowState::Minimised {
                    restore: Restore::Floating(rect()),
                },
                false,
            ),
        ];
        for (name, state, want) in table {
            assert_eq!(framed(&state), want, "{name}");
            let drawn = want || name == "in the dock";
            assert_eq!(drawn_state(&state), drawn, "{name} drawn");
        }
    }
}
