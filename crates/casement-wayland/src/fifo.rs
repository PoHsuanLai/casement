//! `wp_fifo_v1`: a surface's commits wait for the previous one to be presented.

use crate::presentation::Presented;
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::{Event, OutputKey};
use smithay::reexports::wayland_protocols::wp::fifo::v1::server::wp_fifo_manager_v1::WpFifoManagerV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::fifo::{FifoManagerData, FifoManagerState};

/// `wp_fifo_manager_v1`.
#[derive(Debug)]
pub struct FifoPart {
    /// smithay's state.
    pub state: FifoManagerState,
}

impl FifoPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> FifoPart
    where
        D: GlobalDispatch<WpFifoManagerV1, FifoManagerData>
            + Dispatch<WpFifoManagerV1, FifoManagerData>
            + 'static,
    {
        let _ = role_filter;
        FifoPart {
            state: FifoManagerState::new::<D>(display),
        }
    }
}

/// A committed surface set a fifo barrier.
pub fn on_barrier(part: &mut FifoPart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H1: fifo::on_barrier")
}

/// `output` presented a frame: barriers of the surfaces on it are signalled.
pub fn apply_presented(
    part: &mut FifoPart,
    output: &OutputKey,
    presented: Presented,
) -> Result<(), WaylandFault> {
    let _ = (part, output, presented);
    todo!("P0-H1: fifo::apply_presented")
}
