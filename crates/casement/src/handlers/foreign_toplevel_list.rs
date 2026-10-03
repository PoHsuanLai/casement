//! `ext_foreign_toplevel_list_v1` (P0-H8).

use crate::state::State;
use smithay::wayland::foreign_toplevel_list::{
    ForeignToplevelListHandler, ForeignToplevelListState,
};

impl ForeignToplevelListHandler for State {
    fn foreign_toplevel_list_state(&mut self) -> &mut ForeignToplevelListState {
        &mut self.wl.foreign_toplevel_list.state
    }
}
