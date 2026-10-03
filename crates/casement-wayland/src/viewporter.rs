//! `wp_viewporter`: crop and scale. smithay applies viewports in its surface state; casement
//! adds nothing at P0.

use crate::states::GlobalFilter;
use smithay::reexports::wayland_protocols::wp::viewporter::server::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::viewporter::{ViewportState, ViewporterState};

/// `wp_viewporter`.
#[derive(Debug)]
pub struct ViewporterPart {
    /// smithay's state.
    pub state: ViewporterState,
}

impl ViewporterPart {
    /// Creates the `wp_viewporter` global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> ViewporterPart
    where
        D: GlobalDispatch<WpViewporter, GlobalData>
            + Dispatch<WpViewporter, GlobalData>
            + Dispatch<WpViewport, ViewportState>
            + 'static,
    {
        let _ = role_filter;
        ViewporterPart {
            state: ViewporterState::new::<D>(display),
        }
    }
}
