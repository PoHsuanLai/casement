//! zwlr_layer_shell_v1 for the stand-in: surfaces mapped on the one output, arranged and given
//! their initial configure on commit. Shape after anvil's (MIT; THIRD-PARTY.md).

use super::StandIn;
use smithay::desktop::{LayerSurface, WindowSurfaceType, layer_map_for_output};
use smithay::output::Output;
use smithay::reexports::wayland_server::protocol::{wl_output::WlOutput, wl_surface::WlSurface};
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::wlr_layer::{
    Layer, LayerSurface as WlrLayerSurface, LayerSurfaceData, WlrLayerShellHandler,
    WlrLayerShellState,
};

impl WlrLayerShellHandler for StandIn {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell
    }

    fn new_layer_surface(
        &mut self,
        surface: WlrLayerSurface,
        wl_output: Option<WlOutput>,
        _layer: Layer,
        namespace: String,
    ) {
        let output = wl_output
            .as_ref()
            .and_then(Output::from_resource)
            .or_else(|| self.space.outputs().next().cloned());
        let Some(output) = output else { return };
        let _ = layer_map_for_output(&output).map_layer(&LayerSurface::new(surface, namespace));
    }

    fn layer_destroyed(&mut self, surface: WlrLayerSurface) {
        for output in self.space.outputs() {
            let mut map = layer_map_for_output(output);
            let layer = map
                .layers()
                .find(|layer| layer.layer_surface() == &surface)
                .cloned();
            if let Some(layer) = layer {
                map.unmap_layer(&layer);
            }
        }
    }
}

impl StandIn {
    /// On commit of a layer surface: arrange the output's layers, then send the initial
    /// configure if it was not sent yet.
    pub(super) fn commit_layer(&mut self, surface: &WlSurface) {
        let Some(output) = self
            .space
            .outputs()
            .find(|o| {
                layer_map_for_output(o)
                    .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                    .is_some()
            })
            .cloned()
        else {
            return;
        };
        let sent = with_states(surface, |states| {
            states
                .data_map
                .get::<LayerSurfaceData>()
                .is_some_and(|d| d.lock().is_ok_and(|d| d.initial_configure_sent))
        });
        let mut map = layer_map_for_output(&output);
        map.arrange();
        if !sent && let Some(layer) = map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL) {
            layer.layer_surface().send_configure();
        }
    }
}
