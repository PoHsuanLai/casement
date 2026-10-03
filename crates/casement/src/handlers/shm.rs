//! `wl_shm` (P0-H1).

use crate::state::State;
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
        let _ = buffer;
        todo!("P0-H1: BufferHandler::buffer_destroyed")
    }
}
