//! `ext_foreign_toplevel_list_v1`: every window, for docks, switchers and taskbars outside
//! the shell. Hidden from sandboxed clients (I7). Identifiers are chosen by casement.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::WindowId;
use casement_core::policy::GlobalKind;
use casement_core::window::Window;
use smithay::reexports::wayland_protocols::ext::foreign_toplevel_list::v1::server::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::foreign_toplevel_list::{
    ForeignToplevelHandle, ForeignToplevelListGlobalData, ForeignToplevelListHandler, ForeignToplevelListState,
};
use std::collections::BTreeMap;

/// `ext_foreign_toplevel_list_v1` and the handle of each listed window.
#[derive(Debug)]
pub struct ForeignToplevelListPart {
    /// smithay's state.
    pub state: ForeignToplevelListState,
    /// Each listed window's handle.
    pub handles: BTreeMap<WindowId, ForeignToplevelHandle>,
}

impl ForeignToplevelListPart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> ForeignToplevelListPart
    where
        D: ForeignToplevelListHandler
            + GlobalDispatch<ExtForeignToplevelListV1, ForeignToplevelListGlobalData>,
    {
        ForeignToplevelListPart {
            state: ForeignToplevelListState::new_with_filter::<D>(
                display,
                role_filter.for_global(GlobalKind::ForeignToplevelList),
            ),
            handles: BTreeMap::new(),
        }
    }
}

/// Lists `window` (or sends its changed title and app id).
pub fn apply_toplevel(
    part: &mut ForeignToplevelListPart,
    window: &Window,
) -> Result<(), WaylandFault> {
    let _ = (part, window);
    todo!("P0-H8: foreign_toplevel_list::apply_toplevel")
}

/// Withdraws `window`.
pub fn apply_closed(
    part: &mut ForeignToplevelListPart,
    window: WindowId,
) -> Result<(), WaylandFault> {
    let _ = (part, window);
    todo!("P0-H8: foreign_toplevel_list::apply_closed")
}
