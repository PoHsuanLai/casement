//! `zwp_idle_inhibit_manager_v1` (P0-H8).

use crate::state::State;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::idle_inhibit::IdleInhibitHandler;

impl IdleInhibitHandler for State {
    fn inhibit(&mut self, surface: WlSurface) {
        let _ = surface;
        todo!("P0-H8: IdleInhibitHandler::inhibit")
    }

    fn uninhibit(&mut self, surface: WlSurface) {
        let _ = surface;
        todo!("P0-H8: IdleInhibitHandler::uninhibit")
    }
}
