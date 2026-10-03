//! `zwp_input_method_manager_v2` (P0-H7).

use crate::state::State;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Rectangle};
use smithay::wayland::input_method::{InputMethodHandler, PopupSurface};

impl InputMethodHandler for State {
    fn new_popup(&mut self, surface: PopupSurface) {
        let _ = surface;
        todo!("P0-H7: InputMethodHandler::new_popup")
    }

    fn dismiss_popup(&mut self, surface: PopupSurface) {
        let _ = surface;
        todo!("P0-H7: InputMethodHandler::dismiss_popup")
    }

    fn popup_repositioned(&mut self, surface: PopupSurface) {
        let _ = surface;
        todo!("P0-H7: InputMethodHandler::popup_repositioned")
    }

    fn parent_geometry(&self, parent: &WlSurface) -> Rectangle<i32, Logical> {
        let _ = parent;
        todo!("P0-H7: InputMethodHandler::parent_geometry")
    }
}
