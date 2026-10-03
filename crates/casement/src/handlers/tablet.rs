//! `zwp_tablet_manager_v2` (P0-H4).

use crate::state::State;
use smithay::input::tablet::TabletSeatHandler;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

impl TabletSeatHandler for State {
    type ToolFocus = WlSurface;
}
