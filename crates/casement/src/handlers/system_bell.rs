//! `xdg_system_bell_v1` (P0-H2).

use crate::state::State;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::xdg_system_bell::XdgSystemBellHandler;

impl XdgSystemBellHandler for State {
    fn ring(&mut self, surface: Option<WlSurface>) {
        let _ = surface;
        todo!("P0-H2: XdgSystemBellHandler::ring")
    }
}
