//! `zwp_primary_selection_device_manager_v1` (P0-H5).

use crate::state::State;
use smithay::wayland::selection::primary_selection::{
    PrimarySelectionHandler, PrimarySelectionState,
};

impl PrimarySelectionHandler for State {
    fn primary_selection_state(&mut self) -> &mut PrimarySelectionState {
        &mut self.wl.primary_selection.state
    }
}
