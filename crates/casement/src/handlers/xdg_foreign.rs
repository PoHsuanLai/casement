//! `zxdg_exporter_v2`, `zxdg_importer_v2` (P0-H2).

use crate::state::State;
use smithay::wayland::xdg_foreign::{XdgForeignHandler, XdgForeignState};

impl XdgForeignHandler for State {
    fn xdg_foreign_state(&mut self) -> &mut XdgForeignState {
        &mut self.wl.xdg_foreign.state
    }
}
