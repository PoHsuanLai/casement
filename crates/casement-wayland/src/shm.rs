//! `wl_shm`: shared-memory buffers.

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::reexports::wayland_server::protocol::wl_shm::{Format, WlShm};
use smithay::reexports::wayland_server::protocol::wl_shm_pool::WlShmPool;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New,
};
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::shm::{ShmHandler, ShmPoolUserData, ShmState};
use smithay::wayland::{GlobalData, GlobalDispatch2};

/// The version `wl_shm` is served at.
const VERSION: u32 = 2;

/// The pixel formats every client may assume, in the order a bind announces them. smithay
/// announces from a `HashSet`, whose order changes from one run to the next, and no hash order
/// may reach the wire (B17), so the global is ours and the announcement is this list.
pub const ADVERTISED: [Format; 2] = [Format::Argb8888, Format::Xrgb8888];

/// `wl_shm`.
#[derive(Debug)]
pub struct ShmPart {
    /// smithay's state (its pools and buffers; the formats it checks a buffer against).
    pub state: ShmState,
}

/// The user data of the `wl_shm` global: a bind announces [`ADVERTISED`] in order, and the
/// object it makes is smithay's, so pools and buffers are smithay's too.
#[derive(Debug, Clone, Copy)]
pub struct ShmGlobal;

impl<D> GlobalDispatch2<WlShm, D> for ShmGlobal
where
    D: Dispatch<WlShm, GlobalData> + 'static,
{
    fn bind(
        &self,
        _state: &mut D,
        _display: &DisplayHandle,
        _client: &Client,
        resource: New<WlShm>,
        data_init: &mut DataInit<'_, D>,
    ) {
        let shm = data_init.init(resource, GlobalData);
        for format in ADVERTISED {
            shm.format(format);
        }
    }
}

impl ShmPart {
    /// Creates the `wl_shm` global (version 2) with the two formats every client may assume
    /// (ARGB8888, XRGB8888). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> ShmPart
    where
        D: GlobalDispatch<WlShm, GlobalData>
            + GlobalDispatch<WlShm, ShmGlobal>
            + Dispatch<WlShm, GlobalData>
            + Dispatch<WlShmPool, ShmPoolUserData>
            + BufferHandler
            + ShmHandler
            + 'static,
    {
        let _ = role_filter;
        let state = ShmState::new::<D>(display, ADVERTISED);
        // smithay's own global announces in hash order: ours takes its place.
        display.remove_global::<D>(state.global());
        display.create_global::<D, WlShm, _>(VERSION, ShmGlobal);
        ShmPart { state }
    }
}

/// A client buffer was destroyed (`BufferHandler`). The textures made from it live in each
/// surface's renderer state, which smithay resets when the surface commits another buffer or
/// is destroyed, so nothing here holds one and nothing reaches the model.
pub fn on_buffer_destroyed(part: &mut ShmPart, buffer: &WlBuffer) -> Vec<Event> {
    let _ = (part, buffer);
    Vec::new()
}
