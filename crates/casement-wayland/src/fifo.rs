//! `wp_fifo_v1`: a surface's commits wait for the previous one to be presented.

use crate::presentation::Presented;
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::{Event, OutputKey};
use smithay::reexports::wayland_protocols::wp::fifo::v1::server::wp_fifo_manager_v1::WpFifoManagerV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{
    Client, Dispatch, DisplayHandle, GlobalDispatch, Resource, Weak,
};
use smithay::wayland::compositor::{Barrier, with_states};
use smithay::wayland::fifo::{FifoBarrierCachedState, FifoManagerData, FifoManagerState};

/// `wp_fifo_manager_v1`.
#[derive(Debug)]
pub struct FifoPart {
    /// smithay's state.
    pub state: FifoManagerState,
    /// Barriers set by applied commits, signalled at the next presented frame.
    waiting: Vec<Waiting>,
    /// Clients whose blocked commits a signalled barrier may have released
    /// ([`take_cleared`]).
    cleared: Vec<Client>,
}

/// A barrier and the surface whose content update set it.
#[derive(Debug)]
struct Waiting {
    surface: Weak<WlSurface>,
    barrier: Barrier,
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
            waiting: Vec::new(),
            cleared: Vec::new(),
        }
    }
}

/// A committed surface set a fifo barrier: the barrier its applied content update carries is
/// kept until a frame has been presented. (A commit that waits on an earlier barrier is held
/// back by smithay until that one is signalled.)
pub fn on_barrier(part: &mut FifoPart, surface: &WlSurface) -> Vec<Event> {
    let barrier = with_states(surface, |states| {
        states
            .cached_state
            .get::<FifoBarrierCachedState>()
            .current()
            .barrier
            .take()
    });
    if let Some(barrier) = barrier
        && !part
            .waiting
            .iter()
            .any(|waiting| waiting.barrier == barrier)
    {
        part.waiting.push(Waiting {
            surface: surface.downgrade(),
            barrier,
        });
    }
    Vec::new()
}

/// `output` presented a frame: every waiting barrier is signalled (P0 presents each surface on
/// the first output, A28, so whichever output frames first releases it). The clients that
/// had commits held back are listed for [`take_cleared`].
pub fn apply_presented(
    part: &mut FifoPart,
    output: &OutputKey,
    presented: Presented,
) -> Result<(), WaylandFault> {
    let _ = (output, presented);
    for waiting in std::mem::take(&mut part.waiting) {
        waiting.barrier.signal();
        let owner = waiting
            .surface
            .upgrade()
            .ok()
            .and_then(|surface| surface.client());
        if let Some(client) = owner
            && !part.cleared.iter().any(|known| known.id() == client.id())
        {
            part.cleared.push(client);
        }
    }
    Ok(())
}

/// The clients whose held-back commits a barrier signalled since the last call may have
/// released: the caller tells each one's smithay compositor state (`blocker_cleared`).
pub fn take_cleared(part: &mut FifoPart) -> Vec<Client> {
    std::mem::take(&mut part.cleared)
}

/// `surface` was destroyed: its barriers go with it.
pub fn forget_surface(part: &mut FifoPart, surface: &WlSurface) {
    part.waiting
        .retain(|waiting| waiting.surface != surface.downgrade());
}
