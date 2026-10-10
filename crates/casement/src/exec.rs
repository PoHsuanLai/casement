//! The one place effects happen: each `Effect` reaches the protocol module, the backend, the
//! renderer or the clock that carries it out. A failure is logged and degrades; it never
//! panics the loop (CONVENTIONS addendum 2).
//!
//! A batch's `Relayout`s collapse into one arrangement, made after the rest of the batch.
//! Some effects have no wire function to call yet and are logged: `Route` (the press's
//! delivery is the seat's and was decided before the effect), `Disconnect` (a client key names
//! no connection until the client table exists), `Bell` and `Attention` (P3).

mod call;
mod gamma;
mod publish;
mod spawn;

use crate::core_loop;
use crate::state::State;
use casement_core::output::OutputConfig;
use casement_core::{Cause, CoreNote, Effect, KeyboardTarget, LayerKey, OutputKey};
use casement_core::{InputSerial, geometry::LogicalSize, lock::LockReply};
use casement_wayland::output::OutputWire;
use casement_wayland::{
    WaylandFault, foreign_toplevel_list, layer_shell, output, pointer_constraints, presentation,
    quire_window, seat, session_lock, text_input, wlr_toplevel, xdg_shell,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{IsAlive, SERIAL_COUNTER};

/// Carries out `effects` in order.
pub fn exec(state: &mut State, effects: Vec<Effect>) {
    let (relayouts, rest): (Vec<Effect>, Vec<Effect>) = effects
        .into_iter()
        .partition(|effect| matches!(effect, Effect::Relayout(_)));
    for effect in rest {
        apply(state, effect);
    }
    let cause = relayouts
        .into_iter()
        .fold(None, |chosen, effect| match effect {
            Effect::Relayout(cause) => Some(stronger(chosen, cause)),
            _ => chosen,
        });
    if let Some(cause) = cause {
        core_loop::relayout(state, &cause);
    }
}

/// The cause that decides how windows travel when several asked at once: the first that is
/// not a client's own configure.
fn stronger(chosen: Option<Cause>, next: Cause) -> Cause {
    match chosen {
        Some(Cause::Configure) | None => next,
        Some(first) => first,
    }
}

/// One effect.
fn apply(state: &mut State, effect: Effect) {
    match effect {
        Effect::Configure { window, configure } => {
            log(xdg_shell::apply_configure(
                &mut state.wl.xdg_shell,
                window,
                &configure,
            ));
        }
        Effect::ListWindow(window) => {
            if let Some(held) = state.core.model.windows().get(&window) {
                log(foreign_toplevel_list::apply_toplevel(
                    &mut state.wl.foreign_toplevel_list,
                    held,
                ));
            }
        }
        Effect::UnlistWindow(window) => {
            log(foreign_toplevel_list::apply_closed(
                &mut state.wl.foreign_toplevel_list,
                window,
            ));
            log(wlr_toplevel::apply_closed(
                &mut state.wl.wlr_toplevel,
                window,
            ));
        }
        Effect::SendClose { window } => {
            log(xdg_shell::apply_close(&mut state.wl.xdg_shell, window));
        }
        Effect::Keyboard(target) => keyboard(state, &target),
        Effect::Emphasis(list) => {
            for (window, emphasis) in list {
                log(quire_window::apply_emphasis(
                    &mut state.wl.quire_window,
                    window,
                    emphasis,
                ));
            }
        }
        Effect::Ping { client } => log(xdg_shell::apply_ping(&mut state.wl.xdg_shell, client)),
        Effect::ArmTimer(due) => state.clock.arm(&state.event_loop, due),
        Effect::CancelTimer(key) => state.clock.cancel(&key),
        Effect::ConfigureLayer { layer, size } => configure_layer(state, layer, size),
        Effect::Lock(reply) => lock(state, reply),
        Effect::ApplyOutput { output, config } => apply_output(state, &output, &config),
        Effect::Note(note) => note_of(&note),
        // The stack lives in the model and the renderer reads it at the next frame; a frame
        // is drawn at every vblank, so there is nothing to ask for.
        Effect::Raise(_) | Effect::RequestFrames(_) => {}
        Effect::Bell { .. } | Effect::Attention { .. } => {
            tracing::debug!("the bell and attention have no compositor-side effect before P3");
        }
        Effect::StartGrab(grab) => core_loop::start_grab(state, grab),
        Effect::Route(route) => tracing::debug!(?route, "a press route is the seat's to carry"),
        Effect::Disconnect { client, fault } => {
            tracing::warn!(
                client = client.0,
                ?fault,
                "a client to disconnect has no connection"
            );
        }
        Effect::Relayout(cause) => core_loop::relayout(state, &cause),
        // P1-Z's effects: the model emits none of them until the fill lanes land, so these arms
        // are not reached before then.
        Effect::CallBinding(call) => call::call_binding(state, call),
        Effect::Spawn(argv) => spawn::spawn_program(state, argv),
        Effect::SetGamma(output, ramp) => gamma::set_gamma(state, output, ramp),
        Effect::PersistSpaces => publish::persist_spaces(state),
        Effect::PublishWorkspaces => publish::publish_workspaces(state),
        Effect::PublishToplevel(window) => publish::publish_toplevel(state, window),
        // `Effect` grows in core; one this build does not know is logged, never a panic.
        other => tracing::warn!(?other, "an effect this build does not apply"),
    }
}

/// Logs a protocol module's failure: the loop degrades, it does not stop.
fn log(result: Result<(), WaylandFault>) {
    if let Err(fault) = result {
        tracing::warn!(%fault, "an effect failed");
    }
}

/// A diagnostic from the model.
fn note_of(note: &CoreNote) {
    match note {
        CoreNote::RefusedActivation(window) => {
            tracing::debug!(window = window.0, "an activation was refused");
        }
        CoreNote::StaleTimer(key) => tracing::debug!(?key, "a timer fired for something gone"),
        CoreNote::UnknownWindow(window) => {
            tracing::debug!(window = window.0, "an event named an unknown window");
        }
    }
}

/// The surface the model's keyboard target names.
fn focus_surface(state: &State, target: &KeyboardTarget) -> Option<WlSurface> {
    match target {
        KeyboardTarget::Window(window) => state
            .wl
            .xdg_shell
            .toplevels
            .get(window)
            .map(|toplevel| toplevel.wl_surface().clone()),
        KeyboardTarget::Layer(layer) => state.wl.layer_shell.surface_of(*layer).cloned(),
        KeyboardTarget::LockSurface(output) => state
            .wl
            .session_lock
            .surfaces
            .get(output)
            .map(|lock| lock.wl_surface().clone()),
        KeyboardTarget::Nothing => None,
    }
}

/// Moves keyboard focus to what the model says holds it.
fn keyboard(state: &mut State, target: &KeyboardTarget) {
    let focus = focus_surface(state, target);
    crate::handlers::dismiss_popups_for_focus(state, focus.as_ref());
    let serial = InputSerial(SERIAL_COUNTER.next_serial().into());
    let handle = state.wl.seat.seat.clone();
    let followed = focus.clone();
    let left = state.wl.seat.keyboard.current_focus();
    log(seat::apply_keyboard_focus(&handle, state, focus, serial));
    log(text_input::apply_text_focus(&handle, followed.as_ref()));
    let pointer = state.wl.seat.pointer.clone();
    pointer_constraints::on_keyboard_focus(&pointer, left.as_ref(), followed.as_ref());
}

/// A popup's grab ended (a press outside it, or the client closed it): the keyboard grab is
/// dropped and the keyboard goes where the model says. Smithay hands it to the popup's root
/// surface, which may be a bar that never asked for the keyboard.
pub fn end_popup_keyboard(state: &mut State) {
    let keyboard = state.wl.seat.keyboard.clone();
    keyboard.unset_grab(state);
    let target = state.core.model.keyboard_target();
    self::keyboard(state, &target);
}

/// A client left: when the seat's keyboard is not where the model says it is, it goes there.
/// Smithay drops a popup grab whose client died before the popup's destruction reaches us, so
/// `end_popup_keyboard` never ran: the seat is left holding a dead surface, or keyed to
/// something the model does not name, and the app that should have the keys gets none. A live
/// grab holding a live surface is left alone.
pub fn resync_keyboard(state: &mut State) {
    let wanted = focus_surface(state, &state.core.model.keyboard_target());
    let keyboard = &state.wl.seat.keyboard;
    let held = keyboard.current_focus();
    let dead = held.as_ref().is_some_and(|surface| !surface.alive());
    let adrift = !keyboard.is_grabbed() && held != wanted;
    if dead || adrift {
        end_popup_keyboard(state);
    }
}

fn configure_layer(state: &mut State, layer: LayerKey, size: LogicalSize) {
    log(layer_shell::apply_layer_configure(
        &mut state.wl.layer_shell,
        layer,
        size,
    ));
}

/// Carries out the lock's reply.
fn lock(state: &mut State, reply: LockReply) {
    let covers = &mut state.render.chrome.covers;
    match reply {
        LockReply::SendLocked => log(session_lock::apply_locked(&mut state.wl.session_lock)),
        LockReply::ShowCover(output) if !covers.contains(&output) => covers.push(output),
        LockReply::ShowCover(_) => {}
        LockReply::HideCover(output) => covers.retain(|covered| *covered != output),
        LockReply::Finished => session_lock::apply_finished(&mut state.wl.session_lock),
    }
}

/// Puts `config` on `output`: the backend, the vblank clock (at the new rate) and `wl_output`.
fn apply_output(state: &mut State, output: &OutputKey, config: &OutputConfig) {
    let Some(plugged) = state.core.model.outputs().0.get(output) else {
        return;
    };
    let facts = plugged.facts.clone();
    if let casement_backend::BackendKind::Headless(backend) = &mut state.backend {
        backend.apply(&facts.name, config);
    }
    state
        .clock
        .plug_output(&state.event_loop, output.clone(), config.mode.refresh);
    let wire = OutputWire {
        key: output.clone(),
        name: facts.name,
        edid: facts.edid,
        mode: config.mode,
        scale: config.scale,
        transform: config.transform,
        position: config.position,
        physical: facts.physical,
    };
    let advertised = state.wl.output.outputs.contains_key(output);
    let result = match advertised {
        true => output::apply_change(&mut state.wl.output, &wire),
        false => output::apply_plug::<State>(&mut state.wl.output, &state.display, &wire),
    };
    log(result);
    core_loop::resend_scale(state, output, config.scale);
    if let Some(global) = state.wl.output.outputs.get(output) {
        presentation::learn_output(&mut state.wl.presentation, output.clone(), global.clone());
    }
}

/// Withdraws an unplugged output's `wl_output`.
pub(crate) fn withdraw_output(state: &mut State, output: &OutputKey) {
    presentation::forget_output(&mut state.wl.presentation, output);
    if state.wl.output.outputs.contains_key(output) {
        log(output::apply_unplug(
            &mut state.wl.output,
            &state.display,
            output,
        ));
    }
}
