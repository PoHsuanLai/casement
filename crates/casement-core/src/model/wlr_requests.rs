//! Requests from window-list clients (`zwlr_foreign_toplevel_handle_v1`). P1-M2 fills it.

use super::Model;
use super::moved::Moved;
use crate::ids::WindowId;
use crate::window::WlrToplevelRequest;

impl Model {
    /// A window-list client asked something of `window`: the same transitions the window's own
    /// requests take (`request`), plus the minimise rectangle, which is stored for the genie.
    pub(super) fn wlr_request(self, window: WindowId, request: WlrToplevelRequest) -> Moved {
        let _ = (self, window, request);
        todo!("P1-M2: Model::wlr_request")
    }
}
