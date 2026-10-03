//! `zwp_pointer_constraints_v1`: locked and confined pointers (games, remote desktops).

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::input::SeatHandler;
use smithay::input::pointer::PointerHandle;
use smithay::reexports::wayland_protocols::wp::pointer_constraints::zv1::server::{
    zwp_confined_pointer_v1::ZwpConfinedPointerV1, zwp_locked_pointer_v1::ZwpLockedPointerV1,
    zwp_pointer_constraints_v1::ZwpPointerConstraintsV1,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::pointer_constraints::{PointerConstraintUserData, PointerConstraintsState};

/// `zwp_pointer_constraints_v1`.
#[derive(Debug)]
pub struct PointerConstraintsPart {
    /// smithay's state.
    pub state: PointerConstraintsState,
}

impl PointerConstraintsPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> PointerConstraintsPart
    where
        D: GlobalDispatch<ZwpPointerConstraintsV1, GlobalData>
            + Dispatch<ZwpPointerConstraintsV1, GlobalData>
            + Dispatch<ZwpConfinedPointerV1, PointerConstraintUserData<D>>
            + Dispatch<ZwpLockedPointerV1, PointerConstraintUserData<D>>
            + SeatHandler
            + 'static,
    {
        let _ = role_filter;
        PointerConstraintsPart {
            state: PointerConstraintsState::new::<D>(display),
        }
    }
}

/// `surface` asked to lock or confine `pointer`; it activates when the surface has pointer
/// focus.
pub fn on_constraint<D: SeatHandler + 'static>(
    part: &mut PointerConstraintsPart,
    surface: &WlSurface,
    pointer: &PointerHandle<D>,
) -> Vec<Event> {
    let _ = (part, surface, pointer);
    todo!("P0-H4: pointer_constraints::on_constraint")
}
