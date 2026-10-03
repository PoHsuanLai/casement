//! `wl_seat` (P0-H4). Focus targets are plain surfaces at P0 (no compositor-drawn focusable
//! chrome until P3).

use crate::state::State;
use smithay::input::{SeatHandler, SeatState};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

impl SeatHandler for State {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<State> {
        &mut self.wl.seat.state
    }
}
