//! `zxdg_exporter_v2` and `zxdg_importer_v2`: parenting across clients (a portal dialog over
//! an app). smithay serves export and import itself; an imported parent reaches the model
//! through `xdg_shell::on_parent`, so this module has no request functions (the plan listed
//! `on_export`/`on_import`: FINDINGS).

use crate::states::GlobalFilter;
use smithay::reexports::wayland_protocols::xdg::foreign::zv2::server::{
    zxdg_exporter_v2::ZxdgExporterV2, zxdg_importer_v2::ZxdgImporterV2,
};
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::xdg_foreign::{XdgForeignHandler, XdgForeignState};

/// `zxdg_exporter_v2` and `zxdg_importer_v2`.
#[derive(Debug)]
pub struct XdgForeignPart {
    /// smithay's state.
    pub state: XdgForeignState,
}

impl XdgForeignPart {
    /// Creates both globals (version 1). Every connection sees them.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> XdgForeignPart
    where
        D: XdgForeignHandler
            + GlobalDispatch<ZxdgExporterV2, GlobalData>
            + GlobalDispatch<ZxdgImporterV2, GlobalData>,
    {
        let _ = role_filter;
        XdgForeignPart {
            state: XdgForeignState::new::<D>(display),
        }
    }
}
