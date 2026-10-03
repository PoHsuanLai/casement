//! `ext_session_lock_manager_v1` (P0-H3).

use crate::state::State;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::wayland::session_lock::{
    LockSurface, SessionLockHandler, SessionLockManagerState, SessionLocker,
};

impl SessionLockHandler for State {
    fn lock_state(&mut self) -> &mut SessionLockManagerState {
        &mut self.wl.session_lock.state
    }

    fn lock(&mut self, confirmation: SessionLocker) {
        let _ = confirmation;
        todo!("P0-H3: SessionLockHandler::lock")
    }

    fn unlock(&mut self) {
        todo!("P0-H3: SessionLockHandler::unlock")
    }

    fn new_surface(&mut self, surface: LockSurface, output: WlOutput) {
        let _ = (surface, output);
        todo!("P0-H3: SessionLockHandler::new_surface")
    }
}
