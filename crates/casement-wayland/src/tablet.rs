//! `zwp_tablet_manager_v2`: drawing tablets, delivered through smithay's tablet seat.

use crate::states::GlobalFilter;
use smithay::input::tablet::TabletSeatHandler;
use smithay::reexports::wayland_protocols::wp::tablet::zv2::server::{
    zwp_tablet_manager_v2::ZwpTabletManagerV2, zwp_tablet_seat_v2::ZwpTabletSeatV2,
    zwp_tablet_tool_v2::ZwpTabletToolV2,
};
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::tablet_manager::{
    TabletManagerState, TabletSeatUserData, TabletToolUserData,
};

/// `zwp_tablet_manager_v2`.
#[derive(Debug)]
pub struct TabletPart {
    /// smithay's state.
    pub state: TabletManagerState,
}

impl TabletPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> TabletPart
    where
        D: GlobalDispatch<ZwpTabletManagerV2, GlobalData>
            + Dispatch<ZwpTabletManagerV2, GlobalData>
            + Dispatch<ZwpTabletSeatV2, TabletSeatUserData<D>>
            + Dispatch<ZwpTabletToolV2, TabletToolUserData<D>>
            + TabletSeatHandler
            + 'static,
    {
        let _ = role_filter;
        TabletPart {
            state: TabletManagerState::new::<D>(display),
        }
    }
}
