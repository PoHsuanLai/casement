//! `zwp_primary_selection_device_manager_v1`: the middle-click selection. Focus moves with
//! `data_device::apply_selection_focus`.

use crate::states::GlobalFilter;
use casement_core::policy::GlobalKind;
use smithay::reexports::wayland_protocols::wp::primary_selection::zv1::server::zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::selection::primary_selection::{
    PrimaryDeviceManagerGlobalData, PrimarySelectionHandler, PrimarySelectionState,
};

/// `zwp_primary_selection_device_manager_v1`.
#[derive(Debug)]
pub struct PrimarySelectionPart {
    /// smithay's state.
    pub state: PrimarySelectionState,
}

impl PrimarySelectionPart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> PrimarySelectionPart
    where
        D: GlobalDispatch<ZwpPrimarySelectionDeviceManagerV1, PrimaryDeviceManagerGlobalData>
            + PrimarySelectionHandler
            + 'static,
    {
        PrimarySelectionPart {
            state: PrimarySelectionState::new_with_filter::<D, _>(
                display,
                role_filter.for_global(GlobalKind::PrimarySelection),
            ),
        }
    }
}
