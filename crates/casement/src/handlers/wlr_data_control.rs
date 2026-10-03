//! `zwlr_data_control_manager_v1` (P0-H5).

use crate::state::State;
use smithay::wayland::selection::wlr_data_control::{DataControlHandler, DataControlState};

impl DataControlHandler for State {
    fn data_control_state(&mut self) -> &mut DataControlState {
        &mut self.wl.wlr_data_control.state
    }
}
