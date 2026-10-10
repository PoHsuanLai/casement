//! What the idle machine hears from the loop: every input event of any origin (a virtual
//! keyboard's too, I8), a fired idle timer, and which idle inhibitors are visible (a surface
//! counts only while the person can see it: a window on a shown Space, not minimised).

use crate::exec;
use crate::state::State;
use casement_core::time::{Mono, TimerKey};
use casement_core::{Effect, Event};
use casement_wayland::idle_notify::{self, Inhibited};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::get_parent;

/// Tells the idle machine what `events` were, at `now`, then re-reads the inhibitors.
pub(super) fn hear(state: &mut State, events: &[Event], now: Mono) {
    for event in events {
        let effects = match event {
            Event::Input(_) => idle_notify::on_activity(&mut state.wl.idle_notify, now),
            Event::TimerFired(TimerKey::IdleNotify(client, id)) => {
                idle_notify::on_timer(&mut state.wl.idle_notify, *client, *id, now)
            }
            _ => Vec::new(),
        };
        run(state, effects);
    }
    sync_inhibitors(state);
}

/// What the inhibitors say now: held off while one of them is visible. The idle machine hears
/// only a change.
pub(crate) fn sync_inhibitors(state: &mut State) {
    let now = state.clock.now();
    state.wl.idle_inhibit.forget_dead();
    let inhibited = visible_inhibitors(state);
    let effects = idle_notify::on_inhibit(&mut state.wl.idle_notify, inhibited, now);
    run(state, effects);
}

fn run(state: &mut State, effects: Vec<Effect>) {
    if !effects.is_empty() {
        exec::exec(state, effects);
    }
}

fn visible_inhibitors(state: &State) -> Inhibited {
    state
        .wl
        .idle_inhibit
        .inhibited_where(|surface| visible(state, surface))
}

/// Whether the person can see `surface`: a toplevel's tree is visible when its window is shown;
/// any other surface (a layer, a surface with no role yet) counts as visible.
fn visible(state: &State, surface: &WlSurface) -> bool {
    let root = root_of(surface);
    match state.wl.xdg_shell.window_of_surface(&root) {
        Some(window) => state.core.model.shows(window),
        None => true,
    }
}

/// The root of `surface`'s subsurface tree.
fn root_of(surface: &WlSurface) -> WlSurface {
    std::iter::successors(Some(surface.clone()), get_parent)
        .last()
        .unwrap_or_else(|| surface.clone())
}
