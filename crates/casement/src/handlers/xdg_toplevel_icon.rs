//! `xdg_toplevel_icon_manager_v1` (P0-H2).

use crate::core_loop;
use crate::state::State;
use casement_wayland::xdg_toplevel_icon::on_icon;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::XdgToplevel;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::xdg_toplevel_icon::XdgToplevelIconHandler;

impl XdgToplevelIconHandler for State {
    fn set_icon(&mut self, toplevel: XdgToplevel, wl_surface: WlSurface) {
        let events = on_icon(&mut self.wl.xdg_toplevel_icon, &toplevel, &wl_surface);
        core_loop::feed(self, events);
    }
}
