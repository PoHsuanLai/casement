//! `ext_background_effect_manager_v1`: a surface asks for the blur behind a region (C33: the
//! only blur protocol casement serves). Stored at P0 and drawn by blur v0.

use crate::states::GlobalFilter;
use casement_core::Event;
use casement_core::geometry::LogicalRect;
use smithay::reexports::wayland_protocols::ext::background_effect::v1::server::ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::background_effect::{BackgroundEffectState, ExtBackgroundEffectHandler};

/// `ext_background_effect_manager_v1`.
#[derive(Debug)]
pub struct BackgroundEffectPart {
    /// smithay's state.
    pub state: BackgroundEffectState,
}

impl BackgroundEffectPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> BackgroundEffectPart
    where
        D: ExtBackgroundEffectHandler + GlobalDispatch<ExtBackgroundEffectManagerV1, GlobalData>,
    {
        let _ = role_filter;
        BackgroundEffectPart {
            state: BackgroundEffectState::new::<D>(display),
        }
    }
}

/// `surface` set its blur region (surface-local logical rectangles), or cleared it with an
/// empty one.
pub fn on_blur_region(
    part: &mut BackgroundEffectPart,
    surface: &WlSurface,
    region: Vec<LogicalRect>,
) -> Vec<Event> {
    let _ = (part, surface, region);
    todo!("P0-H8: background_effect::on_blur_region")
}
