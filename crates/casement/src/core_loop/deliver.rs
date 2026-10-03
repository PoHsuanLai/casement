//! Input reaching the clients: each event the backend queued is checked against the reach
//! table, hit-tested against the scene, and handed to the seat; presses are routed by the model
//! first (03 section 1.2) and key chords are resolved against the bindings before any client
//! sees the key.
//!
//! Known gaps (their fixes need a field on `State`, an interface ask): the release of a press
//! the model swallowed, or of a key a binding took, is still delivered.

use super::scene::{Hit, hit_test, stack};
use crate::exec;
use crate::state::State;
use casement_core::focus::{HitPart, Press};
use casement_core::geometry::{LogicalPoint, SubPoint, SubPx};
use casement_core::input::{
    AxisFrame, BindingAction, Chord, Edge, InputEvent, InputKind, InputTarget, InputUse, KeyCode,
    Keysym, Origin, Reach, Resolved, TouchStep, resolve,
};
use casement_core::lock::LockState;
use casement_core::time::Mono;
use casement_core::{Effect, InputSerial, KeyboardTarget, Model};
use casement_protocols::vocab::MacMods;
use casement_wayland::ClientState;
use casement_wayland::seat::{self, GestureDelivery, PointerDelivery, SurfaceAt};
use casement_wayland::serial::{PressKind, PressRecord};
use casement_wayland::shortcuts_inhibit::{self, Bindings};
use smithay::backend::input::{InputTime, KeyState, Keycode};
use smithay::input::keyboard::{FilterResult, ModifiersState};
use smithay::input::{Seat, pointer::PointerHandle};
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::SERIAL_COUNTER;

/// Delivers one event. A refused or unhandled event is dropped (a delivery that fails is
/// logged: the loop degrades, it does not stop).
pub(super) fn deliver(state: &mut State, event: &InputEvent) {
    let seat = state.wl.seat.seat.clone();
    match &event.kind {
        InputKind::Key { code, edge } => key(state, &seat, event, *code, *edge),
        InputKind::Motion { delta } => {
            let at = pointer_at(&seat)
                .map(|at| (f64::from(at.0) + px(delta.x), f64::from(at.1) + px(delta.y)));
            if let Some(at) = at {
                motion(state, &seat, event, at, *delta);
            }
        }
        InputKind::MotionTo { at } => {
            let at = (f64::from(at.x), f64::from(at.y));
            motion(state, &seat, event, at, zero());
        }
        InputKind::Button { button, edge } => pressed(state, &seat, event, *button, *edge),
        InputKind::Axis(frame) => scroll(state, &seat, event, *frame),
        InputKind::Swipe(step) => gesture(state, &seat, event, GestureDelivery::Swipe(*step)),
        InputKind::Pinch(step) => gesture(state, &seat, event, GestureDelivery::Pinch(*step)),
        InputKind::Hold(step) => gesture(state, &seat, event, GestureDelivery::Hold(*step)),
        InputKind::Touch(step) => touch(state, &seat, event, *step),
        // A lid or tablet-mode switch moves no pointer and no key: its meaning (idle, the
        // laptop's lid) is P1's.
        InputKind::Switch(_) => {}
    }
}

fn zero() -> SubPoint {
    SubPoint {
        x: SubPx(0),
        y: SubPx(0),
    }
}

/// Sub-pixels as logical pixels.
fn px(sub: SubPx) -> f64 {
    f64::from(sub.0) / 120.0
}

/// A logical position as global sub-pixels.
fn sub_of(at: (f64, f64)) -> SubPoint {
    let sub = |v: f64| {
        SubPx(
            (v * 120.0)
                .round()
                .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32,
        )
    };
    SubPoint {
        x: sub(at.0),
        y: sub(at.1),
    }
}

/// Where the seat's pointer is.
fn pointer_at(seat: &Seat<State>) -> Option<(i32, i32)> {
    let pointer: PointerHandle<State> = seat.get_pointer()?;
    let at = pointer.current_location();
    Some((at.x.floor() as i32, at.y.floor() as i32))
}

fn time_of(at: Mono) -> InputTime {
    InputTime::from_micros(u64::try_from(at.0.as_micros()).unwrap_or(u64::MAX))
}

/// Whether the reach table refuses `event` on `target` for `use_`.
fn refused(state: &State, event: &InputEvent, target: &InputTarget, use_: InputUse) -> bool {
    !matches!(
        state.core.model.reach(&event.origin, target, use_),
        Reach::Allowed
    )
}

/// What a hit lands on, as the reach table names it.
fn target_of(hit: Option<&Hit>) -> InputTarget {
    match hit {
        Some(Hit::Window(window)) => InputTarget::Window(*window),
        Some(Hit::Layer(layer)) => InputTarget::Layer(*layer),
        Some(Hit::Lock(_)) => InputTarget::LockSurface,
        None => InputTarget::Nothing,
    }
}

/// The client a surface belongs to, in our keys.
fn client_of(surface: &WlSurface) -> Option<casement_core::ClientKey> {
    surface
        .client()
        .and_then(|client| ClientState::of(&client).map(|data| data.key))
}

/// Remembers a press the seat delivered so a later request can prove it answers one.
fn remember(
    state: &mut State,
    surface: Option<&WlSurface>,
    serial: InputSerial,
    at: Mono,
    kind: PressKind,
    origin: Origin,
) {
    if let Some(client) = surface.and_then(client_of) {
        state.wl.seat.record_press(
            serial,
            PressRecord {
                client,
                at,
                kind,
                origin,
            },
        );
    }
}

fn next_serial() -> InputSerial {
    InputSerial(SERIAL_COUNTER.next_serial().into())
}

/// The modifiers as the Mac view of the keyboard.
fn mac_mods(mods: &ModifiersState) -> MacMods {
    [
        (mods.shift, MacMods::SHIFT),
        (mods.ctrl, MacMods::CONTROL),
        (mods.alt, MacMods::OPTION),
        (mods.logo, MacMods::COMMAND),
        (mods.caps_lock, MacMods::CAPS_LOCK),
    ]
    .into_iter()
    .filter(|(held, _)| *held)
    .fold(MacMods::NONE, |all, (_, one)| all.with(one))
}

fn keyboard_target(model: &Model) -> InputTarget {
    match model.keyboard_target() {
        KeyboardTarget::Window(window) => InputTarget::Window(window),
        KeyboardTarget::Layer(layer) => InputTarget::Layer(layer),
        KeyboardTarget::LockSurface(_) => InputTarget::LockSurface,
        KeyboardTarget::Nothing => InputTarget::Nothing,
    }
}

/// A key: a chord that is a binding is carried out and not delivered; any other key goes to
/// the keyboard focus. While a client inhibits shortcuts, or the session is locked, no chord
/// resolves.
fn key(state: &mut State, seat: &Seat<State>, event: &InputEvent, code: KeyCode, edge: Edge) {
    if refused(
        state,
        event,
        &keyboard_target(&state.core.model),
        InputUse::Key,
    ) {
        return;
    }
    let Some(keyboard) = seat.get_keyboard() else {
        return;
    };
    let live = matches!(state.core.model.lock(), LockState::Unlocked)
        && shortcuts_inhibit::bindings(seat) == Bindings::Active
        && edge == Edge::Press;
    let bindings = state.core.model.settings().bindings.clone();
    let origin = event.origin;
    let (keycode, key_state) = (
        Keycode::new(code.0.saturating_add(8)),
        match edge {
            Edge::Press => KeyState::Pressed,
            Edge::Release => KeyState::Released,
        },
    );
    let serial = SERIAL_COUNTER.next_serial();
    let taken = keyboard.input(
        state,
        keycode,
        key_state,
        serial,
        time_of(event.at),
        |_, mods, handle| {
            let chord = handle.raw_latin_sym_or_raw_current_sym().map(|sym| Chord {
                mods: mac_mods(mods),
                key: Keysym(sym.raw()),
            });
            match chord.map(|chord| resolve(&bindings, &chord, &origin)) {
                Some(Resolved::Action(action)) if live => FilterResult::Intercept(action),
                _ => FilterResult::Forward,
            }
        },
    );
    match taken {
        Some(action) => carry_out(state, &action),
        None if edge == Edge::Press => {
            let focus = keyboard.current_focus();
            remember(
                state,
                focus.as_ref(),
                InputSerial(serial.into()),
                event.at,
                PressKind::Key,
                origin,
            );
        }
        None => {}
    }
}

/// A binding's action. A desktop command is the model's; a D-Bus call or a program waits for
/// the services that carry them (P2).
fn carry_out(state: &mut State, action: &BindingAction) {
    match action {
        BindingAction::Desktop(command) => {
            let command = *command;
            let effects = run_model(state, |model, now| model.command(command, now));
            exec::exec(state, effects);
        }
        BindingAction::Call(_) | BindingAction::Spawn(_) => {
            tracing::info!(?action, "a binding that calls out is carried out from P2");
        }
    }
}

/// Runs a model entry point at the clock's now; the effects it asks for.
fn run_model(
    state: &mut State,
    step: impl FnOnce(Model, Mono) -> (Model, Vec<Effect>),
) -> Vec<Effect> {
    let now = state.clock.now();
    let standing_in = super::standing_in(now);
    let model = std::mem::replace(&mut state.core.model, standing_in);
    let (model, effects) = step(model, now);
    state.core.model = model;
    effects
}

/// The desktop's extent: the pointer stays inside the union of the outputs.
fn clamp(state: &State, at: (f64, f64)) -> Option<(f64, f64)> {
    let rects: Vec<_> = state
        .core
        .model
        .outputs()
        .0
        .values()
        .map(|output| output.config.logical_rect())
        .collect();
    let left = rects
        .iter()
        .map(|r| f64::from(r.origin.x))
        .reduce(f64::min)?;
    let top = rects
        .iter()
        .map(|r| f64::from(r.origin.y))
        .reduce(f64::min)?;
    let right = rects
        .iter()
        .map(|r| f64::from(r.origin.x) + f64::from(r.size.w))
        .reduce(f64::max)?;
    let bottom = rects
        .iter()
        .map(|r| f64::from(r.origin.y) + f64::from(r.size.h))
        .reduce(f64::max)?;
    Some((at.0.clamp(left, right - 1.0), at.1.clamp(top, bottom - 1.0)))
}

fn logical(at: (f64, f64)) -> LogicalPoint {
    LogicalPoint {
        x: at.0.floor() as i32,
        y: at.1.floor() as i32,
    }
}

/// The pointer moves to `at` (global logical, clamped to the desktop).
fn motion(
    state: &mut State,
    seat: &Seat<State>,
    event: &InputEvent,
    at: (f64, f64),
    delta: SubPoint,
) {
    let Some(at) = clamp(state, at) else {
        return;
    };
    let under = hit_test(&stack(state), logical(at));
    if refused(
        state,
        event,
        &target_of(under.as_ref().map(|(hit, _)| hit)),
        InputUse::Motion,
    ) {
        return;
    }
    state.render.chrome.cursor_at = logical(at);
    let delivery = PointerDelivery::Motion {
        focus: under.map(|(_, surface)| surface),
        at: sub_of(at),
        delta,
        time: event.at,
    };
    log(seat::apply_pointer(seat, state, delivery));
}

/// A pointer button. A press on a window is routed first: it may activate and raise the window
/// and be delivered, or only activate and be swallowed.
fn pressed(
    state: &mut State,
    seat: &Seat<State>,
    event: &InputEvent,
    button: casement_core::input::ButtonCode,
    edge: Edge,
) {
    let Some(here) = pointer_at(seat) else {
        return;
    };
    let under = hit_test(
        &stack(state),
        LogicalPoint {
            x: here.0,
            y: here.1,
        },
    );
    let target = target_of(under.as_ref().map(|(hit, _)| hit));
    if refused(state, event, &target, InputUse::Press) {
        return;
    }
    let surface = under.as_ref().map(|(_, at)| at.surface.clone());
    if edge == Edge::Press
        && let Some((Hit::Window(window), _)) = &under
        && !deliver_press(state, seat, event, *window)
    {
        return;
    }
    let serial = next_serial();
    log(seat::apply_pointer(
        seat,
        state,
        PointerDelivery::Button {
            button,
            edge,
            serial,
            time: event.at,
        },
    ));
    if edge == Edge::Press {
        remember(
            state,
            surface.as_ref(),
            serial,
            event.at,
            PressKind::Button,
            event.origin,
        );
    }
}

/// Routes a press on `window` through the model; whether the press is delivered to the client.
fn deliver_press(
    state: &mut State,
    seat: &Seat<State>,
    event: &InputEvent,
    window: casement_core::WindowId,
) -> bool {
    let mods = seat.get_keyboard().map_or(MacMods::NONE, |keyboard| {
        mac_mods(&keyboard.modifier_state())
    });
    let press = Press {
        window,
        part: HitPart::Content,
        mods,
        origin: event.origin,
    };
    let effects = run_model(state, |model, now| model.press(press, now));
    let route = effects.iter().find_map(|effect| match effect {
        Effect::Route(route) => Some(*route),
        _ => None,
    });
    exec::exec(state, effects);
    use casement_core::focus::PressRoute as Route;
    !matches!(
        route,
        Some(Route::ActivateRaiseSwallow | Route::SwallowWithBell | Route::MoveWithoutActivation)
    )
}

/// One scroll frame to the client under the pointer.
fn scroll(state: &mut State, seat: &Seat<State>, event: &InputEvent, frame: AxisFrame) {
    if let Some(under) = under_pointer(state, seat)
        && !refused(state, event, &target_of(Some(&under.0)), InputUse::Scroll)
    {
        log(seat::apply_axis(seat, state, frame, event.at));
    }
}

/// A gesture step to the client under the pointer.
fn gesture(state: &mut State, seat: &Seat<State>, event: &InputEvent, step: GestureDelivery) {
    if let Some(under) = under_pointer(state, seat)
        && !refused(
            state,
            event,
            &target_of(Some(&under.0)),
            InputUse::SystemGesture,
        )
    {
        log(seat::apply_gesture(
            seat,
            state,
            step,
            next_serial(),
            event.at,
        ));
    }
}

/// What is under the pointer now.
fn under_pointer(state: &State, seat: &Seat<State>) -> Option<(Hit, SurfaceAt)> {
    let here = pointer_at(seat)?;
    hit_test(
        &stack(state),
        LogicalPoint {
            x: here.0,
            y: here.1,
        },
    )
}

/// A touch step; a touch going down is hit-tested where it lands.
fn touch(state: &mut State, seat: &Seat<State>, event: &InputEvent, step: TouchStep) {
    let under = match step {
        TouchStep::Down { at, .. } => hit_test(&stack(state), at),
        _ => None,
    };
    let target = target_of(under.as_ref().map(|(hit, _)| hit));
    let use_ = match step {
        TouchStep::Down { .. } | TouchStep::Up { .. } => InputUse::Press,
        _ => InputUse::Motion,
    };
    if refused(state, event, &target, use_) {
        return;
    }
    let serial = next_serial();
    let surface = under.as_ref().map(|(_, at)| at.surface.clone());
    let focus = under.map(|(_, at)| at);
    log(seat::apply_touch(
        seat, state, step, focus, serial, event.at,
    ));
    if matches!(step, TouchStep::Down { .. }) {
        remember(
            state,
            surface.as_ref(),
            serial,
            event.at,
            PressKind::Touch,
            event.origin,
        );
    }
}

fn log(result: Result<(), casement_wayland::WaylandFault>) {
    if let Err(fault) = result {
        tracing::warn!(%fault, "an input delivery failed");
    }
}

/// The pointer has not moved, but what is under it may have (a surface mapped, unmapped or
/// moved): the surface now there gets the pointer (A16).
pub(super) fn refocus(state: &mut State) {
    let seat = state.wl.seat.seat.clone();
    let Some(here) = pointer_at(&seat) else {
        return;
    };
    let under = hit_test(
        &stack(state),
        LogicalPoint {
            x: here.0,
            y: here.1,
        },
    )
    .map(|(_, at)| at);
    let now = state.clock.now();
    log(seat::apply_refocus(&seat, state, under, now));
}
