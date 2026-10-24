//! `quire_window_manager_v1` (P0-H2, C50).

use crate::core_loop;
use crate::state::State;
use casement_core::{Event, InputSerial, WindowId};
use casement_protocols::vocab::{Emphasis, PlacementMark};
use casement_wayland::quire_window::{QuireWindowHandler, QuireWindowPart, WindowFacts};
use casement_wayland::xdg_shell::{GrabUse, base_role, grab_proven};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::XdgToplevel;
use smithay::reexports::wayland_server::Client;

impl QuireWindowHandler for State {
    fn quire_window_part(&mut self) -> &mut QuireWindowPart {
        &mut self.wl.quire_window
    }

    fn window_of_toplevel(&self, toplevel: &XdgToplevel) -> Option<WindowId> {
        self.wl.xdg_shell.window_of_toplevel(toplevel)
    }

    fn quire_window_events(&mut self, events: Vec<Event>) {
        // A root a client claims is checked before the model adopts it (PORTAL-15).
        let events = crate::a11y::claimed(self, events);
        let (early, now): (Vec<Event>, Vec<Event>) = events
            .into_iter()
            .partition(|event| before_map(self, event));
        for event in early {
            if let Event::WindowHint { window, .. } = &event {
                let held = self.wl.quire_window.early.entry(*window).or_default();
                held.push(event);
            }
        }
        core_loop::feed(self, now);
    }

    fn window_facts(&self, window: WindowId) -> Option<WindowFacts> {
        let surface = self
            .wl
            .xdg_shell
            .toplevels
            .get(&window)?
            .wl_surface()
            .clone();
        let (emphasis, mark) = self
            .core
            .model
            .windows()
            .get(&window)
            .map_or((Emphasis::Inactive, PlacementMark::Free), |held| {
                (held.emphasis, held.state.mark())
            });
        Some(WindowFacts {
            emphasis,
            mark,
            base_role: base_role(&self.wl.xdg_shell, window),
            surface,
        })
    }

    fn press_proves(&self, client: &Client, serial: InputSerial) -> bool {
        grab_proven(self.press_of(client, serial).as_ref(), GrabUse::AnyPress)
    }
}

/// Whether `event` is a hint about a window the model does not hold yet (it has not mapped).
fn before_map(state: &State, event: &Event) -> bool {
    matches!(event, Event::WindowHint { window, .. }
        if !state.core.model.windows().contains_key(window))
}

/// The hints each window in `events` that just mapped said before it did, fed now that the
/// model holds it.
pub(crate) fn replay_early(state: &mut State, events: &[Event]) {
    let early: Vec<Event> = events
        .iter()
        .filter_map(|event| match event {
            Event::ToplevelMapped(mapped) => state.wl.quire_window.early.remove(&mapped.window),
            _ => None,
        })
        .flatten()
        .collect();
    core_loop::feed(state, early);
}
