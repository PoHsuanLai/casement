//! `wp_fractional_scale_v1`: the exact scale a surface should draw at, sent before its first
//! configure (shell-host FINDINGS 167).

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::Event;
use ds_core::geometry::scale::Scale;
use smithay::reexports::wayland_protocols::wp::fractional_scale::v1::server::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::WpFractionalScaleV1,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::fractional_scale::{
    FractionalScaleData, FractionalScaleHandler, FractionalScaleManagerState,
};

/// `wp_fractional_scale_manager_v1`.
#[derive(Debug)]
pub struct FractionalScalePart {
    /// smithay's state.
    pub state: FractionalScaleManagerState,
}

impl FractionalScalePart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> FractionalScalePart
    where
        D: GlobalDispatch<WpFractionalScaleManagerV1, GlobalData>
            + Dispatch<WpFractionalScaleManagerV1, GlobalData>
            + Dispatch<WpFractionalScaleV1, FractionalScaleData>
            + FractionalScaleHandler
            + 'static,
    {
        let _ = role_filter;
        FractionalScalePart {
            state: FractionalScaleManagerState::new::<D>(display),
        }
    }
}

/// A surface asked for its preferred scale: the handler answers with [`apply_scale`] for the
/// output it will appear on, before the surface's first configure.
pub fn on_preferred(part: &mut FractionalScalePart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H1: fractional_scale::on_preferred")
}

/// Sends `scale` as `surface`'s preferred scale (and the matching integer buffer scale).
pub fn apply_scale(
    part: &mut FractionalScalePart,
    surface: &WlSurface,
    scale: Scale,
) -> Result<(), WaylandFault> {
    let _ = (part, surface, scale);
    todo!("P0-H1: fractional_scale::apply_scale")
}
