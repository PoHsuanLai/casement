//! Input reaching the clients: each event the backend queued is checked against the reach
//! table, hit-tested against the scene, and handed to the seat; presses are routed by the model
//! first (03 section 1.2) and key chords are resolved against the bindings before any client
//! sees the key.
//!
//! A press the model swallowed, a press the reach table refused and a key a binding took are
//! remembered (`seat.taken`), so their releases are not delivered either. Keys wait at the
//! text-input gate while an input method is about to grab the keyboard.

use super::agent_gate;
use super::agent_wait;
use super::divider;
use super::frames;
use super::grab;
use super::overview;
use super::scene::{Hit, hit_test, stack};
use crate::exec;
use crate::state::State;
use casement_core::focus::{HitPart, Press};
use casement_core::geometry::{LogicalPoint, SubPoint, SubPx};
use casement_core::input::{
    AxisFrame, BindingAction, Chord, DesktopCommand, InputEvent, InputKind, InputTarget, InputUse,
    KeyCode, Keysym, Origin, TouchStep, resolve,
};
use casement_core::lock::LockState;
use casement_core::shell::ShellNote;
use casement_core::time::{Due, Mono, TimerKey};
use casement_core::{Effect, InputSerial, KeyboardTarget, Model, SwipeOwner};
use casement_protocols::vocab::HeldMods;
use casement_protocols::vocab::PressEdge;
use casement_wayland::seat::{
    self, GestureDelivery, KeyDelivery, KeyForward, KeyRoute, PointerDelivery, PressFate, Pressed,
    SurfaceAt,
};
use casement_wayland::serial::{PressKind, PressRecord};
use casement_wayland::shortcuts_inhibit;
use casement_wayland::{ClientState, text_input};
use smithay::input::keyboard::{FilterResult, ModifiersState};
use smithay::input::{Seat, pointer::PointerHandle};
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::SERIAL_COUNTER;

/// The Escape key's keysym.
const ESCAPE: u32 = 0xff1b;

/// Delivers one event. A refused or unhandled event is dropped (a delivery that fails is
/// logged: the loop degrades, it does not stop).
pub(super) fn deliver(state: &mut State, event: &InputEvent) {
    match event.origin {
        // An agent lease's event meets the gate first, and may wait there (`agent_wait`).
        Origin::Agent(lease) => agent_wait::arrive(state, lease, event),
        _ => through(state, event),
    }
}

/// An event past the agent gate (or one that never meets it): the rest of delivery.
pub(super) fn through(state: &mut State, event: &InputEvent) {
    let seat = state.wl.seat.seat.clone();
    if let Origin::Agent(lease) = event.origin {
        agent_gate::note(state, lease, &event.kind);
    }
    // Mission Control keeps scrolls, gestures and touches from the windows.
    let others = matches!(
        event.kind,
        InputKind::Axis(_)
            | InputKind::Swipe(_)
            | InputKind::Pinch(_)
            | InputKind::Hold(_)
            | InputKind::Touch(_)
    );
    if others && overview::takes_input(state) {
        return;
    }
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
        // A swipe the desktop took (the Spaces, Mission Control) is not the clients'.
        InputKind::Swipe(step) => match state.core.model.swipe_owner() {
            SwipeOwner::Client => gesture(state, &seat, event, GestureDelivery::Swipe(*step)),
            SwipeOwner::Desktop => {}
        },
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
pub(super) fn pointer_at(seat: &Seat<State>) -> Option<(i32, i32)> {
    let pointer: PointerHandle<State> = seat.get_pointer()?;
    let at = pointer.current_location();
    Some((at.x.floor() as i32, at.y.floor() as i32))
}

/// Whether the reach table refuses `event` on `target` for `use_`.
fn refused(
    state: &State,
    event: &InputEvent,
    target: &InputTarget,
    use_: InputUse,
    at: Option<LogicalPoint>,
) -> bool {
    match state.core.model.reach(&event.origin, target, use_) {
        Ok(()) => false,
        Err(why) => {
            if let Origin::Agent(lease) = event.origin {
                // At info, with what was hit at `at` (none for a key) and what the lease holds: a refused agent
                // click is otherwise silent, and the two keys are what tell why. `cursor` is
                // where the person's pointer was last drawn, which a refused move never changes.
                tracing::info!(
                    lease = lease.0,
                    ?why,
                    ?use_,
                    ?target,
                    held = ?state.core.model.leases().window_of(lease),
                    ?at,
                    cursor = ?state.render.chrome.cursor_at,
                    "agent input refused by the reach table"
                );
            }
            true
        }
    }
}

/// A press of `button` that no client gets: its release is not delivered either. An agent
/// lease's press is taken off the lease's (`agent_gate::held_back`), never put in the person's.
fn take_button(state: &mut State, event: &InputEvent, button: casement_core::input::ButtonCode) {
    if !agent_gate::held_back(state, event.origin, Pressed::Button(button)) {
        state.wl.seat.taken = std::mem::take(&mut state.wl.seat.taken).take_button(button);
    }
}

/// Whether `event` is an agent lease's release. It is delivered wherever the seat sends it, past
/// the reach table: the gate let it through only because its press was delivered
/// (`agent_gate::note`), and a release held back would leave the key or button down in the
/// client and the seat after the lease ended or its window closed.
fn ends_agent_press(event: &InputEvent, edge: PressEdge) -> bool {
    edge == PressEdge::Release && matches!(event.origin, Origin::Agent(_))
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

pub(super) fn next_serial() -> InputSerial {
    InputSerial(SERIAL_COUNTER.next_serial().into())
}

/// The modifiers as the Mac view of the keyboard.
pub(super) fn held_mods(mods: &ModifiersState) -> HeldMods {
    [
        (mods.shift, HeldMods::SHIFT),
        (mods.ctrl, HeldMods::CONTROL),
        (mods.alt, HeldMods::OPTION),
        (mods.logo, HeldMods::COMMAND),
        (mods.caps_lock, HeldMods::CAPS_LOCK),
    ]
    .into_iter()
    .filter(|(held, _)| *held)
    .fold(HeldMods::NONE, |all, (_, one)| all.with(one))
}

/// The modifiers of a binding's chord: `held_mods` without the locks. Caps Lock is a state, not
/// a key held for the chord (Num Lock is not in `HeldMods` at all), so a binding matches with
/// either on.
fn chord_mods(mods: &ModifiersState) -> HeldMods {
    let held = held_mods(mods);
    HeldMods(held.0 & !HeldMods::CAPS_LOCK.0)
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
fn key(state: &mut State, seat: &Seat<State>, event: &InputEvent, code: KeyCode, edge: PressEdge) {
    if !ends_agent_press(event, edge)
        && refused(
            state,
            event,
            &keyboard_target(&state.core.model),
            InputUse::Key,
            None,
        )
    {
        match edge {
            PressEdge::Press if agent_gate::held_back(state, event.origin, Pressed::Key(code)) => {}
            PressEdge::Press => {
                state.wl.seat.taken = std::mem::take(&mut state.wl.seat.taken).take_key(code);
            }
            PressEdge::Release => {
                state.wl.seat.past =
                    std::mem::take(&mut state.wl.seat.past).refused_release(event.device, code);
            }
        }
        return;
    }
    let delivery = KeyDelivery {
        code,
        edge,
        serial: next_serial(),
        time: event.at,
        origin: event.origin,
        device: event.device,
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
/// is locked, no chord resolves. The release of a key a binding took is not delivered. A copy
/// the input method sends back through its virtual keyboard goes past its grab, straight to the
/// focus (`seat::key_route`): the real key already went through the bindings. On the lock
/// screen a key goes to the lock surface past any input method's grab (`KeyForward::Focus`).
fn deliver_key(state: &mut State, seat: &Seat<State>, key: KeyDelivery) {
    let facts = seat::route_facts(seat, key.code);
    let (past, route) = std::mem::take(&mut state.wl.seat.past).route(&key, facts);
    state.wl.seat.past = past;
    if route == KeyRoute::PastGrab {
        seat::deliver_past_grab(seat, &key);
        if key.edge == PressEdge::Press {
            remember_key_press(state, seat, &key);
        }
        return;
    }
    // A lease's keys are never the person's taken ones (`agent_gate`): its release delivers.
    let fate = match key.origin {
        Origin::Agent(_) => PressFate::Delivered,
        _ => {
            let (taken, fate) = std::mem::take(&mut state.wl.seat.taken).release_key(key.code);
            state.wl.seat.taken = taken;
            fate
        }
    };
    let live = matches!(state.core.model.lock(), LockState::Unlocked)
        && !shortcuts_inhibit::bindings_suspended(seat)
        && key.edge == PressEdge::Press;
    let bindings = state.core.model.settings().bindings.clone();
    let origin = key.origin;
    let forward = match state.core.model.keyboard_target() {
        KeyboardTarget::LockSurface(_) => KeyForward::Focus,
        _ => KeyForward::Grab,
    };
    let answer = seat::apply_key(seat, state, key, forward, |state, mods, handle| {
        if key.edge == PressEdge::Release && fate == PressFate::Taken {
            return FilterResult::Intercept(None);
        }
        // A VT chord is the seat's, not a binding's: it works under a lock and a shortcut
        // inhibitor (P1-T), and only a kernel keyboard can make it (CASW-13): a virtual
        // keyboard's chord is an ordinary key.
        let vt = match origin.switches_vt() {
            true => super::tty::vt_chord(
                state,
                mods,
                &handle.modified_syms(),
                &handle.raw_syms(),
                key.edge,
            ),
            false => None,
        };
        if vt.is_some() {
            if key.edge == PressEdge::Press {
                state.wl.seat.taken = std::mem::take(&mut state.wl.seat.taken).take_key(key.code);
            }
            return FilterResult::Intercept(None);
        }
        // Escape leaves Mission Control (the same key as its command, which toggles).
        let escape = handle
            .raw_latin_sym_or_raw_current_sym()
            .is_some_and(|sym| sym.raw() == ESCAPE)
            && chord_mods(mods) == HeldMods::NONE;
        if live && escape && origin.fires_bindings() && overview::escape_leaves(state) {
            return FilterResult::Intercept(Some(BindingAction::Desktop(
                DesktopCommand::MissionControl,
            )));
        }
        let chord = handle.raw_latin_sym_or_raw_current_sym().map(|sym| Chord {
            mods: chord_mods(mods),
            key: Keysym(sym.raw()),
        });
        match chord.and_then(|chord| resolve(&bindings, &chord, &origin)) {
            Some(action) if live => FilterResult::Intercept(Some(action)),
            _ => FilterResult::Forward,
        }
    });
    match answer {
        Ok(Some(Some(action))) => {
            state.wl.seat.taken = std::mem::take(&mut state.wl.seat.taken).take_key(key.code);
            carry_out(state, &action);
        }
        Ok(Some(None)) => {}
        Ok(None) if key.edge == PressEdge::Press => remember_key_press(state, seat, &key),
        Ok(None) => {}
        Err(fault) => tracing::warn!(%fault, "a key delivery failed"),
    }
    tell_shell_modifiers(state, seat);
}

/// Records a key press the keyboard focus was sent, so its serial answers a real press.
fn remember_key_press(state: &mut State, seat: &Seat<State>, key: &KeyDelivery) {
    let focus = seat
        .get_keyboard()
        .and_then(|keyboard| keyboard.current_focus());
    remember(
        state,
        focus.as_ref(),
        key.serial,
        key.time,
        PressKind::Key,
        key.origin,
        None,
    );
}

/// The keyboard's modifier set, as the shell sees it (it ignores a set it was already sent).
fn tell_shell_modifiers(state: &mut State, seat: &Seat<State>) {
    let mods = seat.get_keyboard().map_or(HeldMods::NONE, |keyboard| {
        held_mods(&keyboard.modifier_state())
    });
    exec::shell_note(state, ShellNote::Modifiers(mods));
}

/// A binding's action. A desktop command is the model's; a D-Bus call and a program go out
/// through the injected seams within the frame the key was pressed in.
fn carry_out(state: &mut State, action: &BindingAction) {
    match action {
        BindingAction::Desktop(command) => {
            let command = *command;
            let effects = run_model(state, |model, now| model.command(command, now));
            exec::exec(state, effects);
        }
        BindingAction::Call(call) => {
            exec::exec(state, vec![Effect::CallBinding(call.clone())]);
        }
        BindingAction::Spawn(argv) => exec::exec(state, vec![Effect::Spawn(argv.clone())]),
    }
}

/// Runs a model entry point at the clock's now; the effects it asks for.
pub(super) fn run_model(
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

/// The pointer moves to `at` while something of the compositor's own takes the motion: the
/// clients under it are left and hear nothing.
fn move_unheard(
    state: &mut State,
    seat: &Seat<State>,
    event: &InputEvent,
    at: (f64, f64),
    delta: SubPoint,
) {
    state.render.chrome.cursor_at = logical(at);
    let delivery = PointerDelivery::Motion {
        focus: None,
        at: sub_of(at),
        delta,
        time: event.at,
    };
    log(seat::apply_pointer(seat, state, delivery));
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
    if grab::follow(state, logical(at)) {
        // A move or resize: the pointer still moves, or the next relative motion would start
        // from the press again and the window would stay pinned near it (the trial login of
        // 2026-10-19).
        move_unheard(state, seat, event, at, delta);
        return;
    }
    if overview::takes_input(state) {
        mission_control_motion(state, seat, event, at, delta);
        return;
    }
    let stacked = stack(state);
    super::reveal::moved(state, &stacked, logical(at));
    if divider::moved(state, &stacked, logical(at)) {
        // The strip between Split View halves (or a drag of it).
        move_unheard(state, seat, event, at, delta);
        return;
    }
    let over = frames::under(state, &stacked, logical(at));
    let under = match over {
        Some(_) => None,
        None => hit_test(&stacked, logical(at)),
    };
    let target = match over {
        Some(over) => InputTarget::Window(over.window),
        None => target_of(under.as_ref().map(|(hit, _)| hit)),
    };
    if refused(state, event, &target, InputUse::Motion, Some(logical(at))) {
        return;
    }
    frames::hover(state, over);
    state.render.chrome.cursor_at = logical(at);
    let delivery = PointerDelivery::Motion {
        focus: under.map(|(_, surface)| surface),
        at: sub_of(at),
        delta,
        time: event.at,
    };
    log(seat::apply_pointer(seat, state, delivery));
}

/// The pointer moves while Mission Control has it: no client is under it, the seat follows it
/// and the window it is over is the hovered one.
fn mission_control_motion(
    state: &mut State,
    seat: &Seat<State>,
    event: &InputEvent,
    at: (f64, f64),
    delta: SubPoint,
) {
    if refused(
        state,
        event,
        &InputTarget::Nothing,
        InputUse::Motion,
        Some(logical(at)),
    ) {
        return;
    }
    overview::hover(state, logical(at));
    state.render.chrome.cursor_at = logical(at);
    let delivery = PointerDelivery::Motion {
        focus: None,
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
    edge: PressEdge,
) {
    let ends_grab = edge == PressEdge::Release
        && !ends_agent_press(event, edge)
        && (grab::ends_with(state, button) || divider::ends_with(state, button));
    press_edge(state, seat, event, button, edge);
    if ends_grab {
        // The grab left the pointer over nothing; it is over what it is over again now, and the
        // client under it is told (after the release, which that client never saw the press of).
        refocus(state);
    }
}

/// A press on a Split View divider starts its drag (when the reach table allows it) and is never
/// delivered to a client; its release is swallowed with it. Whether the press was on one.
fn press_on_divider(
    state: &mut State,
    event: &InputEvent,
    at: LogicalPoint,
    button: casement_core::input::ButtonCode,
) -> bool {
    let stacked = stack(state);
    let Some(found) = divider::under_pointer(state, &stacked, at) else {
        return false;
    };
    take_button(state, event, button);
    let target = InputTarget::Window(found.leading);
    if !refused(state, event, &target, InputUse::Press, Some(at)) {
        divider::begin(state, found, button);
    }
    true
}

fn press_edge(
    state: &mut State,
    seat: &Seat<State>,
    event: &InputEvent,
    button: casement_core::input::ButtonCode,
    edge: PressEdge,
) {
    if edge == PressEdge::Release && !ends_agent_press(event, edge) {
        // An agent lease never starts the compositor's own interactions (`agent_gate`), so its
        // release ends none of the person's.
        overview::released(state);
        grab::end(state, button);
        divider::end(state, button);
    }
    if edge == PressEdge::Release && !ends_agent_press(event, edge) {
        // A lease's release is never the end of a press the person's frame took.
        let (taken, fate) = std::mem::take(&mut state.wl.seat.taken).release_button(button);
        state.wl.seat.taken = taken;
        if fate == PressFate::Taken {
            frames::released(state, seat, button);
            return;
        }
    }
    let Some(here) = pointer_at(seat) else {
        return;
    };
    let at = LogicalPoint {
        x: here.0,
        y: here.1,
    };
    if edge == PressEdge::Press && overview::takes_input(state) {
        // Mission Control takes the press (a pick or a leave); no client sees it or its release.
        take_button(state, event, button);
        if !refused(
            state,
            event,
            &InputTarget::Nothing,
            InputUse::Press,
            Some(at),
        ) {
            overview::press(state, event, at);
        }
        return;
    }
    if edge == PressEdge::Press && press_on_divider(state, event, at, button) {
        return;
    }
    let (over, under) = frames::pick(state, at);
    if let (Some(over), PressEdge::Press) = (over, edge) {
        // A frame's press never reaches a client; its release is swallowed with it.
        take_button(state, event, button);
        let target = InputTarget::Window(over.window);
        if !refused(state, event, &target, InputUse::Press, Some(at)) {
            frames::press(state, seat, event, button, over);
        }
        return;
    }
    let target = target_of(under.as_ref().map(|(hit, _)| hit));
    let surface = under.as_ref().map(|(_, at)| at.surface.clone());
    let delivered = (ends_agent_press(event, edge)
        || !refused(state, event, &target, InputUse::Press, Some(at)))
        && match (&under, edge) {
            (Some((Hit::Window(window), _)), PressEdge::Press) => {
                deliver_press(state, seat, event, *window)
            }
            (Some((Hit::Layer(layer), _)), PressEdge::Press) => {
                let effects = run_model(state, |model, now| model.press_layer(*layer, now));
                exec::exec(state, effects);
                true
            }
            _ => true,
        };
    if !delivered {
        if edge == PressEdge::Press
            && !agent_gate::held_back(state, event.origin, Pressed::Button(button))
        {
            take_button(state, event, button);
        }
        return;
    }
    let serial = next_serial();
    if edge == PressEdge::Press && dismisses_popup_grab(state) {
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
    let (kind, held_by) = match edge {
        PressEdge::Press => (PressKind::Button, surface),
        PressEdge::Release => (
            PressKind::Release,
            seat.get_pointer()
                .and_then(|pointer| pointer.current_focus())
                .and_then(Option::<WlSurface>::from),
        ),
    };
    remember(
        state,
        held_by.as_ref(),
        serial,
        event.at,
        kind,
        event.origin,
        Some(button),
    );
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
    let mods = seat.get_keyboard().map_or(HeldMods::NONE, |keyboard| {
        held_mods(&keyboard.modifier_state())
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
        && !refused(
            state,
            event,
            &target_of(Some(&under.0)),
            InputUse::Scroll,
            seat_point(seat),
        )
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
            seat_point(seat),
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
    frames::pick(state, seat_point(seat)?).1
}

/// Where the seat's pointer is, as a logical point.
fn seat_point(seat: &Seat<State>) -> Option<LogicalPoint> {
    pointer_at(seat).map(|(x, y)| LogicalPoint { x, y })
}

/// A touch step; a touch going down is hit-tested where it lands.
fn touch(state: &mut State, seat: &Seat<State>, event: &InputEvent, step: TouchStep) {
    let down_at = match step {
        TouchStep::Down { at, .. } => Some(at),
        _ => None,
    };
    let under = down_at.and_then(|at| hit_test(&stack(state), at));
    let target = target_of(under.as_ref().map(|(hit, _)| hit));
    let use_ = match step {
        TouchStep::Down { .. } | TouchStep::Up { .. } => InputUse::Press,
        _ => InputUse::Motion,
    };
    if refused(state, event, &target, use_, down_at) {
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
    if state.render.pointer_placed {
        return;
    }
    let Some(centre) = state.core.model.pointer_start() else {
        return;
    };
    state.render.pointer_placed = true;
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
    if state.wl.seat.grab.is_some() || divider::is_dragging(state) {
        // An interactive move or resize holds the pointer: no client is under it until it ends.
        return;
    }
    let seat = state.wl.seat.seat.clone();
    let Some(here) = pointer_at(&seat) else {
        return;
    };
    // Mission Control has the pointer: no window is under it.
    let under = match overview::takes_input(state) {
        true => None,
        false => frames::pick(
            state,
            LogicalPoint {
                x: here.0,
                y: here.1,
            },
        )
        .1
        .map(|(_, at)| at),
    };
    let now = state.clock.now();
    log(seat::apply_refocus(&seat, state, under, now));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (case, state, chord modifiers)
    fn cases() -> Vec<(&'static str, ModifiersState, HeldMods)> {
        let with = |f: fn(&mut ModifiersState)| {
            let mut state = ModifiersState::default();
            f(&mut state);
            state
        };
        vec![
            ("nothing held", with(|_| {}), HeldMods::NONE),
            ("command", with(|m| m.logo = true), HeldMods::COMMAND),
            (
                "caps lock alone",
                with(|m| m.caps_lock = true),
                HeldMods::NONE,
            ),
            (
                "command with caps lock",
                with(|m| {
                    m.logo = true;
                    m.caps_lock = true;
                }),
                HeldMods::COMMAND,
            ),
            (
                "command with num lock",
                with(|m| {
                    m.logo = true;
                    m.num_lock = true;
                }),
                HeldMods::COMMAND,
            ),
            (
                "shift, control and both locks",
                with(|m| {
                    m.shift = true;
                    m.ctrl = true;
                    m.caps_lock = true;
                    m.num_lock = true;
                }),
                HeldMods::SHIFT.with(HeldMods::CONTROL),
            ),
        ]
    }

    #[test]
    fn a_chord_ignores_the_locks() {
        for (case, state, want) in cases() {
            assert_eq!(chord_mods(&state), want, "{case}");
        }
    }
}
