//! `xdg_system_bell_v1` (P0-H2): the bell rings for the app of the surface it names, else for
//! the active app.

use crate::core_loop;
use crate::exec::exec;
use crate::state::State;
use casement_core::Effect;
use casement_wayland::system_bell::{app_to_ring, on_bell};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::xdg_system_bell::XdgSystemBellHandler;

impl XdgSystemBellHandler for State {
    fn ring(&mut self, surface: Option<WlSurface>) {
        let events = on_bell(&mut self.wl.system_bell, surface.as_ref());
        core_loop::feed(self, events);
        let window = surface
            .as_ref()
            .and_then(|surface| self.wl.xdg_shell.window_of_surface(surface));
        if let Some(app) = app_to_ring(&self.core.model, window) {
            exec(self, vec![Effect::Bell { app }]);
        }
    }
}
