//! `zwlr_data_control_manager_v1`: the older clipboard-manager protocol (wl-clipboard and
//! anvil-era tools). Hidden from sandboxed clients (I7).

use crate::data_device::SelectionKind;
use crate::primary_selection::PrimarySelectionPart;
use crate::states::GlobalFilter;
use casement_core::Event;
use casement_core::policy::GlobalKind;
use smithay::reexports::wayland_protocols_wlr::data_control::v1::server::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::selection::wlr_data_control::{DataControlManagerGlobalData, DataControlState};

/// `zwlr_data_control_manager_v1`.
#[derive(Debug)]
pub struct WlrDataControlPart {
    /// smithay's state.
    pub state: DataControlState,
}

impl WlrDataControlPart {
    /// Creates the global (version 2) over the clipboard and `primary`, filtered by the I7
    /// table.
    pub fn new<D>(
        display: &DisplayHandle,
        role_filter: GlobalFilter,
        primary: &PrimarySelectionPart,
    ) -> WlrDataControlPart
    where
        D: GlobalDispatch<ZwlrDataControlManagerV1, DataControlManagerGlobalData> + 'static,
    {
        WlrDataControlPart {
            state: DataControlState::new::<D, _>(
                display,
                Some(&primary.state),
                role_filter.for_global(GlobalKind::WlrDataControl),
            ),
        }
    }
}

/// A data-control client set a selection.
pub fn on_control_selection(part: &mut WlrDataControlPart, kind: SelectionKind) -> Vec<Event> {
    let _ = (part, kind);
    todo!("P0-H5: wlr_data_control::on_control_selection")
}
