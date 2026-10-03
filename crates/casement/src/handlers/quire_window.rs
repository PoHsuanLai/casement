//! `quire_window_manager_v1` (P0-H2, C50).

use crate::core_loop;
use crate::state::State;
use casement_core::{Event, WindowId};
use casement_wayland::quire_window::{QuireWindowHandler, QuireWindowPart};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::XdgToplevel;

impl QuireWindowHandler for State {
    fn quire_window_part(&mut self) -> &mut QuireWindowPart {
        &mut self.wl.quire_window
    }

    fn window_of_toplevel(&self, toplevel: &XdgToplevel) -> Option<WindowId> {
        let _ = toplevel;
        todo!("P0-H2: QuireWindowHandler::window_of_toplevel")
    }

    fn quire_window_events(&mut self, events: Vec<Event>) {
        core_loop::feed(self, events);
    }
}
