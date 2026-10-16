//! `ext_foreign_toplevel_list_v1` (P0-H8). Seeing every window is a power (`Power::Windows`,
//! the same as the window list and the workspaces): smithay asks `foreign_toplevel_list_gate`
//! when a list is bound, announces nothing to a list that waits, and settles it when the answer
//! comes (`consent::release_gate`).

use crate::state::State;
use casement_protocols::vocab::Power;
use casement_wayland::consent;
use smithay::reexports::wayland_server::Client;
use smithay::wayland::Gate;
use smithay::wayland::foreign_toplevel_list::{
    ForeignToplevelListHandler, ForeignToplevelListState,
};

impl ForeignToplevelListHandler for State {
    fn foreign_toplevel_list_state(&mut self) -> &mut ForeignToplevelListState {
        &mut self.wl.foreign_toplevel_list.state
    }

    fn foreign_toplevel_list_gate(&mut self, client: &Client) -> Gate {
        consent::gate(self, client, Power::Windows)
    }
}
