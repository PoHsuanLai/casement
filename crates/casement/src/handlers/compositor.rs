//! `wl_compositor` (P0-H1): every commit enters here and is routed by its surface's role.

use crate::state::State;
use casement_wayland::ClientState;
use smithay::reexports::wayland_server::Client;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::{CompositorClientState, CompositorHandler, CompositorState};

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.wl.compositor.state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        match ClientState::of(client) {
            Some(state) => &state.compositor,
            // Every client is inserted through `clients::insert_client` with a ClientState;
            // a client without one cannot exist, and smithay's signature leaves no error path.
            None => unreachable!("a client inserted without casement's ClientState"),
        }
    }

    fn commit(&mut self, surface: &WlSurface) {
        let _ = surface;
        todo!("P0-H1: CompositorHandler::commit (on_commit, then route by CommitRoute)")
    }
}
