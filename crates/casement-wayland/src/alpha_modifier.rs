//! `wp_alpha_modifier_v1`: a surface's own opacity multiplier, read from its cached state at
//! render time.

use crate::states::GlobalFilter;
use smithay::reexports::wayland_protocols::wp::alpha_modifier::v1::server::{
    wp_alpha_modifier_surface_v1::WpAlphaModifierSurfaceV1, wp_alpha_modifier_v1::WpAlphaModifierV1,
};
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::alpha_modifier::{AlphaModifierState, AlphaModifierSurfaceUserData};

/// `wp_alpha_modifier_v1`.
#[derive(Debug)]
pub struct AlphaModifierPart {
    /// smithay's state.
    pub state: AlphaModifierState,
}

impl AlphaModifierPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> AlphaModifierPart
    where
        D: GlobalDispatch<WpAlphaModifierV1, GlobalData>
            + Dispatch<WpAlphaModifierV1, GlobalData>
            + Dispatch<WpAlphaModifierSurfaceV1, AlphaModifierSurfaceUserData>
            + 'static,
    {
        let _ = role_filter;
        AlphaModifierPart {
            state: AlphaModifierState::new::<D>(display),
        }
    }
}
