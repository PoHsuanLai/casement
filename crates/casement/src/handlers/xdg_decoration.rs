//! `zxdg_decoration_manager_v1` (P0-H2).

use crate::state::State;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
use smithay::wayland::shell::xdg::ToplevelSurface;
use smithay::wayland::shell::xdg::decoration::XdgDecorationHandler;

impl XdgDecorationHandler for State {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        let _ = toplevel;
        todo!("P0-H2: XdgDecorationHandler::new_decoration")
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: Mode) {
        let _ = (toplevel, mode);
        todo!("P0-H2: XdgDecorationHandler::request_mode")
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        let _ = toplevel;
        todo!("P0-H2: XdgDecorationHandler::unset_mode")
    }
}
