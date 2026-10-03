//! `zwlr_layer_shell_v1` (P0-H3).

use crate::state::State;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::wayland::shell::wlr_layer::{
    Layer, LayerSurface, WlrLayerShellHandler, WlrLayerShellState,
};

impl WlrLayerShellHandler for State {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.wl.layer_shell.state
    }

    fn new_layer_surface(
        &mut self,
        surface: LayerSurface,
        output: Option<WlOutput>,
        layer: Layer,
        namespace: String,
    ) {
        let _ = (surface, output, layer, namespace);
        todo!("P0-H3: WlrLayerShellHandler::new_layer_surface")
    }
}
