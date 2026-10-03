//! `ext_data_control_manager_v1`: clipboard managers (sill's clipboard history). Hidden from
//! sandboxed clients (I7).

use crate::data_device::SelectionKind;
use crate::primary_selection::PrimarySelectionPart;
use crate::states::GlobalFilter;
use casement_core::Event;
use casement_core::policy::GlobalKind;
use smithay::reexports::wayland_protocols::ext::data_control::v1::server::ext_data_control_manager_v1::ExtDataControlManagerV1;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::selection::ext_data_control::{DataControlState, ExtDataControlManagerGlobalData};

/// `ext_data_control_manager_v1`.
#[derive(Debug)]
pub struct ExtDataControlPart {
    /// smithay's state.
    pub state: DataControlState,
}

impl ExtDataControlPart {
    /// Creates the global (version 1) over the clipboard and `primary`, filtered by the I7
    /// table.
    pub fn new<D>(
        display: &DisplayHandle,
        role_filter: GlobalFilter,
        primary: &PrimarySelectionPart,
    ) -> ExtDataControlPart
    where
        D: GlobalDispatch<ExtDataControlManagerV1, ExtDataControlManagerGlobalData> + 'static,
    {
        ExtDataControlPart {
            state: DataControlState::new::<D, _>(
                display,
                Some(&primary.state),
                role_filter.for_global(GlobalKind::ExtDataControl),
            ),
        }
    }
}

/// A data-control client set a selection.
pub fn on_control_selection(part: &mut ExtDataControlPart, kind: SelectionKind) -> Vec<Event> {
    let _ = (part, kind);
    todo!("P0-H5: ext_data_control::on_control_selection")
}
