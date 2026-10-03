//! `zwp_pointer_constraints_v1` (P0-H4).

use crate::core_loop;
use crate::state::State;
use casement_wayland::pointer_constraints;
use smithay::input::pointer::PointerHandle;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::pointer_constraints::PointerConstraintsHandler;

impl PointerConstraintsHandler for State {
    fn new_constraint(&mut self, surface: &WlSurface, pointer: &PointerHandle<State>) {
        let events =
            pointer_constraints::on_constraint(&mut self.wl.pointer_constraints, surface, pointer);
        core_loop::feed(self, events);
    }
}
