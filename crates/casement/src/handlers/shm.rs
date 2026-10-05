//! `wl_shm` (P0-H1).

use crate::core_loop;
use crate::state::State;
use casement_wayland::{shm, xdg_toplevel_icon};
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::shm::{ShmHandler, ShmState};

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.wl.shm.state
    }
}

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, buffer: &WlBuffer) {
        xdg_toplevel_icon::on_buffer_destroyed(&self.wl.xdg_toplevel_icon, buffer);
        let events = shm::on_buffer_destroyed(&mut self.wl.shm, buffer);
        if !events.is_empty() {
            core_loop::feed(self, events);
        }
    }
}
