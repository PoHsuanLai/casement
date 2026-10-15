//! The green light's window menu in the loop: the pointer resting on the light is fed to the
//! dwell machine (`casement_core::window::Dwell`), its timer is armed on the model's timer
//! book, and when it says so the window asks for its menu (`ToplevelRequest::WindowMenu`, the
//! same request a client-side frame's `show_window_menu` makes). The shell draws the menu.

use super::feed;
use super::frames::{contents, styles};
use crate::exec;
use crate::state::State;
use casement_core::time::{Due, TimerKey};
use casement_core::window::{DwellInput, DwellOutcome, DwellTiming, Light, LightAt, menu_point};
use casement_core::{Effect, Event, ToplevelRequest};

/// The pointer is over `light` (or over no light) now.
pub(super) fn dwell(state: &mut State, light: Option<LightAt>) {
    let now = state.clock.now();
    step(state, DwellInput::Over(light, now));
}

/// A button went down on a light: it is a click, not a rest.
pub(super) fn pressed(state: &mut State) {
    step(state, DwellInput::Pressed);
}

/// The dwell timer fired in `events`: the rest may be long enough.
pub(super) fn hear(state: &mut State, events: &[Event]) {
    if events
        .iter()
        .any(|event| matches!(event, Event::TimerFired(TimerKey::LightDwell)))
    {
        let now = state.clock.now();
        step(state, DwellInput::Tick(now));
    }
}

fn step(state: &mut State, input: DwellInput) {
    let (dwell, outcome) = state.render.light_dwell.step(DwellTiming::MAC, input);
    state.render.light_dwell = dwell;
    match outcome {
        DwellOutcome::Arm(at) => exec::exec(
            state,
            vec![Effect::ArmTimer(Due {
                at,
                key: TimerKey::LightDwell,
            })],
        ),
        DwellOutcome::Cancel => exec::exec(state, vec![Effect::CancelTimer(TimerKey::LightDwell)]),
        DwellOutcome::Show(on) => show(state, on),
        _ => {}
    }
}

/// `on`'s window asks for its menu just under the green light, when that window still has a
/// frame drawn.
fn show(state: &mut State, on: LightAt) {
    let styles = styles(state);
    let contents = contents(state);
    let Some((style, content)) = styles.get(&on.window).zip(contents.get(&on.window)) else {
        return;
    };
    let at = menu_point(style.around(*content).light(Light::Zoom));
    feed(
        state,
        vec![Event::ToplevelRequest {
            window: on.window,
            request: ToplevelRequest::WindowMenu { at },
        }],
    );
}
