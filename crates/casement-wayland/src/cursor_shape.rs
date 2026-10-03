//! `wp_cursor_shape_manager_v1`: named cursors. smithay turns a shape request into
//! `SeatHandler::cursor_image(Named(..))`, which reaches `seat::on_cursor_image`.

use crate::states::GlobalFilter;
use smithay::input::SeatHandler;
use smithay::reexports::wayland_protocols::wp::cursor_shape::v1::server::wp_cursor_shape_manager_v1::WpCursorShapeManagerV1;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::cursor_shape::CursorShapeManagerState;

/// `wp_cursor_shape_manager_v1`.
#[derive(Debug)]
pub struct CursorShapePart {
    /// smithay's state.
    pub state: CursorShapeManagerState,
}

impl CursorShapePart {
    /// Creates the global (version 2). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> CursorShapePart
    where
        D: GlobalDispatch<WpCursorShapeManagerV1, GlobalData>
            + Dispatch<WpCursorShapeManagerV1, GlobalData>
            + SeatHandler
            + 'static,
    {
        let _ = role_filter;
        CursorShapePart {
            state: CursorShapeManagerState::new::<D>(display),
        }
    }
}
