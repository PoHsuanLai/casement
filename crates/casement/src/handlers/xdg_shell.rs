//! `xdg_wm_base` (P0-H2).

use crate::state::State;
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::utils::Serial;
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
};

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.wl.xdg_shell.state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let _ = surface;
        todo!("P0-H2: XdgShellHandler::new_toplevel")
    }

    fn new_popup(&mut self, surface: PopupSurface, positioner: PositionerState) {
        let _ = (surface, positioner);
        todo!("P0-H2: XdgShellHandler::new_popup")
    }

    fn grab(&mut self, surface: PopupSurface, seat: WlSeat, serial: Serial) {
        let _ = (surface, seat, serial);
        todo!("P0-H2: XdgShellHandler::grab")
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        let _ = (surface, positioner, token);
        todo!("P0-H2: XdgShellHandler::reposition_request")
    }
}
