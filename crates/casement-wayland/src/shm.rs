//! `wl_shm`: shared-memory buffers.

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::reexports::wayland_server::protocol::wl_shm::WlShm;
use smithay::reexports::wayland_server::protocol::wl_shm_pool::WlShmPool;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::shm::{ShmHandler, ShmPoolUserData, ShmState};

/// `wl_shm`.
#[derive(Debug)]
pub struct ShmPart {
    /// smithay's state.
    pub state: ShmState,
}

impl ShmPart {
    /// Creates the `wl_shm` global (version 2) with the two formats every client may assume
    /// (ARGB8888, XRGB8888). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> ShmPart
    where
        D: GlobalDispatch<WlShm, GlobalData>
            + Dispatch<WlShm, GlobalData>
            + Dispatch<WlShmPool, ShmPoolUserData>
            + BufferHandler
            + ShmHandler
            + 'static,
    {
        let _ = role_filter;
        ShmPart {
            state: ShmState::new::<D>(display, Vec::new()),
        }
    }
}

/// A client buffer was destroyed (`BufferHandler`): textures made from it are dropped.
pub fn on_buffer_destroyed(part: &mut ShmPart, buffer: &WlBuffer) -> Vec<Event> {
    let _ = (part, buffer);
    todo!("P0-H1: shm::on_buffer_destroyed")
}
