//! Input reaching the clients: each event the backend queued is checked against the reach
//! table, hit-tested against the scene, and handed to the seat; presses are routed by the model
//! first (03 section 1.2) and key chords are resolved against the bindings before any client
//! sees the key.
//!
//! A press the model swallowed, a press the reach table refused and a key a binding took are
//! remembered (`seat.taken`), so their releases are not delivered either. Keys wait at the
//! text-input gate while an input method is about to grab the keyboard.

use super::grab;
use super::scene::{Hit, hit_test, stack};
use crate::exec;
use crate::state::{PointerStart, State};
use casement_core::focus::{HitPart, Press};
use casement_core::geometry::{LogicalPoint, SubPoint, SubPx};
use casement_core::input::{
    AxisFrame, BindingAction, Chord, Edge, InputEvent, InputKind, InputTarget, InputUse, KeyCode,
    Keysym, Origin, Reach, Resolved, TouchStep, resolve,
};
use casement_core::lock::LockState;
use casement_core::time::{Due, Mono, TimerKey};
use casement_core::{Effect, InputSerial, KeyboardTarget, Model};
use casement_protocols::vocab::MacMods;
use casement_wayland::seat::{
    self, GestureDelivery, KeyDelivery, PointerDelivery, PressFate, SurfaceAt,
};
use casement_wayland::serial::{PressKind, PressRecord};
use casement_wayland::shortcuts_inhibit::{self, Bindings};
use casement_wayland::{ClientState, text_input};
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
    button: Option<casement_core::input::ButtonCode>,
) {
    if let Some(client) = surface.and_then(client_of) {
        state.wl.seat.record_press(
            serial,
            PressRecord {
                client,
                at,
                kind,
                origin,
                button,
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

/// A key waits at the text-input gate while an input method is about to take the keyboard
/// (the first key after a field is enabled must reach the method, not the document); the keys
/// the gate lets go are then delivered ([`deliver_keys`]).
fn key(state: &mut State, seat: &Seat<State>, event: &InputEvent, code: KeyCode, edge: Edge) {
    if refused(
        state,
        event,
        &keyboard_target(&state.core.model),
        InputUse::Key,
    ) {
        if edge == Edge::Press {
            state.wl.seat.taken = std::mem::take(&mut state.wl.seat.taken).take_key(code);
        }
        return;
    }
    let delivery = KeyDelivery {
        code,
        edge,
        serial: next_serial(),
        time: event.at,
        origin: event.origin,
    };
    let now = state.clock.now();
    let ready = text_input::admit_key(&mut state.wl.text_input, seat, delivery, now);
    deliver_keys(state, seat, ready);
    arm_ime_hold(state);
}

/// Asks for a timer at the instant the oldest waiting key may no longer wait.
pub(super) fn arm_ime_hold(state: &mut State) {
    if let Some(at) = state.wl.text_input.gate.due() {
        let due = Due {
            at,
            key: TimerKey::ImeHold,
        };
        exec::exec(state, vec![Effect::ArmTimer(due)]);
    }
}

/// The keys the gate let go, in order.
pub(super) fn deliver_keys(state: &mut State, seat: &Seat<State>, keys: Vec<KeyDelivery>) {
    for key in keys {
        deliver_key(state, seat, key);
    }
}

/// One key, through the keyboard: a chord that is a binding is carried out and not delivered;
/// any other key goes to the keyboard focus. While a client inhibits shortcuts, or the session
/// is locked, no chord resolves. The release of a key a binding took is not delivered.
fn deliver_key(state: &mut State, seat: &Seat<State>, key: KeyDelivery) {
    let (taken, fate) = std::mem::take(&mut state.wl.seat.taken).release_key(key.code);
    state.wl.seat.taken = taken;
    let live = matches!(state.core.model.lock(), LockState::Unlocked)
        && shortcuts_inhibit::bindings(seat) == Bindings::Active
        && key.edge == Edge::Press;
    let bindings = state.core.model.settings().bindings.clone();
    let origin = key.origin;
    let answer = seat::apply_key(seat, state, key, |_, mods, handle| {
        if key.edge == Edge::Release && fate == PressFate::Taken {
            return FilterResult::Intercept(None);
        }
        let chord = handle.raw_latin_sym_or_raw_current_sym().map(|sym| Chord {
            mods: mac_mods(mods),
            key: Keysym(sym.raw()),
        });
        match chord.map(|chord| resolve(&bindings, &chord, &origin)) {
            Some(Resolved::Action(action)) if live => FilterResult::Intercept(Some(action)),
            _ => FilterResult::Forward,
        }
    });
    match answer {
        Ok(Some(Some(action))) => {
            state.wl.seat.taken = std::mem::take(&mut state.wl.seat.taken).take_key(key.code);
            carry_out(state, &action);
        }
        Ok(Some(None)) => {}
        Ok(None) if key.edge == Edge::Press => {
            let focus = seat
                .get_keyboard()
                .and_then(|keyboard| keyboard.current_focus());
            remember(
                state,
                focus.as_ref(),
                key.serial,
                key.time,
                PressKind::Key,
                origin,
                None,
            );
        }
        Ok(None) => {}
        Err(fault) => tracing::warn!(%fault, "a key delivery failed"),
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
    if grab::follow(state, logical(at)) == grab::Follow::Taken {
        return;
    }
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
/// and be delivered, or only activate and be swallowed; a swallowed or refused press takes its
/// release with it.
fn pressed(
    state: &mut State,
    seat: &Seat<State>,
    event: &InputEvent,
    button: casement_core::input::ButtonCode,
    edge: Edge,
) {
    if edge == Edge::Release {
        grab::end(state, button);
        let (taken, fate) = std::mem::take(&mut state.wl.seat.taken).release_button(button);
        state.wl.seat.taken = taken;
        if fate == PressFate::Taken {
            return;
        }
    }
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
    let surface = under.as_ref().map(|(_, at)| at.surface.clone());
    let delivered = !refused(state, event, &target, InputUse::Press)
        && match (&under, edge) {
            (Some((Hit::Window(window), _)), Edge::Press) => {
                deliver_press(state, seat, event, *window)
            }
            _ => true,
        };
    if !delivered {
        if edge == Edge::Press {
            state.wl.seat.taken = std::mem::take(&mut state.wl.seat.taken).take_button(button);
        }
        return;
    }
    let serial = next_serial();
    if edge == Edge::Press && dismisses_popup_grab(state) {
        // Smithay ends the grab at this press and would hand the keyboard to the popup's root
        // (a bar that never asked for it, even for a moment): the model says where it goes.
        exec::end_popup_keyboard(state);
    }
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
            Some(button),
        );
    }
}

/// Whether a press now ends a popup's explicit grab: a popup's grab holds the pointer and the
/// keyboard, and a press that lands on another client than the one holding the keyboard
/// dismisses it (xdg-shell's rule, which smithay's popup pointer grab applies).
fn dismisses_popup_grab(state: &State) -> bool {
    let seat = &state.wl.seat;
    let client_id = |surface: Option<WlSurface>| {
        surface
            .as_ref()
            .and_then(Resource::client)
            .map(|client| client.id())
    };
    seat.pointer.is_grabbed()
        && seat.keyboard.is_grabbed()
        && client_id(seat.pointer.current_focus().and_then(Into::into))
            != client_id(seat.keyboard.current_focus())
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
            None,
        );
    }
}

fn log(result: Result<(), casement_wayland::WaylandFault>) {
    if let Err(fault) = result {
        tracing::warn!(%fault, "an input delivery failed");
    }
}

/// The pointer's first position: the centre of the primary output, the first time there is one.
pub(super) fn start_pointer(state: &mut State) {
    if state.render.pointer == PointerStart::Placed {
        return;
    }
    let Some(centre) = state.core.model.pointer_start() else {
        return;
    };
    state.render.pointer = PointerStart::Placed;
    let at = (f64::from(centre.x), f64::from(centre.y));
    state.render.chrome.cursor_at = logical(at);
    let seat = state.wl.seat.seat.clone();
    let delivery = PointerDelivery::Motion {
        focus: None,
        at: sub_of(at),
        delta: zero(),
        time: state.clock.now(),
    };
    log(seat::apply_pointer(&seat, state, delivery));
    refocus(state);
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
