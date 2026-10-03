//! `wp_single_pixel_buffer_manager_v1`: one-pixel buffers for solid colours.

use crate::states::GlobalFilter;
use smithay::reexports::wayland_protocols::wp::single_pixel_buffer::v1::server::wp_single_pixel_buffer_manager_v1::WpSinglePixelBufferManagerV1;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::single_pixel_buffer::SinglePixelBufferState;

/// `wp_single_pixel_buffer_manager_v1`.
#[derive(Debug)]
pub struct SinglePixelPart {
    /// smithay's state.
    pub state: SinglePixelBufferState,
}

impl SinglePixelPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> SinglePixelPart
    where
        D: GlobalDispatch<WpSinglePixelBufferManagerV1, GlobalData>
            + Dispatch<WpSinglePixelBufferManagerV1, GlobalData>
            + 'static,
    {
        let _ = role_filter;
        SinglePixelPart {
            state: SinglePixelBufferState::new::<D>(display),
        }
    }
}
