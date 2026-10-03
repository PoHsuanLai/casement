//! `zwp_relative_pointer_manager_v1`: unaccelerated deltas, sent by `seat::apply_pointer`.

use crate::states::GlobalFilter;
use smithay::input::SeatHandler;
use smithay::reexports::wayland_protocols::wp::relative_pointer::zv1::server::{
    zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1,
    zwp_relative_pointer_v1::ZwpRelativePointerV1,
};
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::relative_pointer::{RelativePointerManagerState, RelativePointerUserData};

/// `zwp_relative_pointer_manager_v1`.
#[derive(Debug)]
pub struct RelativePointerPart {
    /// smithay's state.
    pub state: RelativePointerManagerState,
}

impl RelativePointerPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> RelativePointerPart
    where
        D: GlobalDispatch<ZwpRelativePointerManagerV1, GlobalData>
            + Dispatch<ZwpRelativePointerManagerV1, GlobalData>
            + Dispatch<ZwpRelativePointerV1, RelativePointerUserData<D>>
            + SeatHandler
            + 'static,
    {
        let _ = role_filter;
        RelativePointerPart {
            state: RelativePointerManagerState::new::<D>(display),
        }
    }
}
