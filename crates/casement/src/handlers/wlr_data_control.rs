//! `zwlr_data_control_manager_v1` (P0-H5). The same power as `ext_data_control`
//! (`Power::Clipboard`), gated the same way.

use crate::state::State;
use casement_protocols::vocab::Power;
use casement_wayland::consent;
use smithay::reexports::wayland_server::Client;
use smithay::wayland::Gate;
use smithay::wayland::selection::wlr_data_control::{DataControlHandler, DataControlState};

impl DataControlHandler for State {
    fn data_control_state(&mut self) -> &mut DataControlState {
        &mut self.wl.wlr_data_control.state
    }

    fn data_control_gate(&mut self, client: &Client) -> Gate {
        consent::gate(self, client, Power::Clipboard)
    }
}
