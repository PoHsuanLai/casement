//! `zwlr_layer_shell_v1` (P0-H3).

use crate::core_loop;
use crate::state::State;
use casement_core::OutputKey;
use casement_wayland::layer_shell::{self, LayerNamespace};
use smithay::desktop::{PopupKind, PopupManager};
use smithay::output::Output;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::wayland::shell::wlr_layer::{
    Layer, LayerSurface, WlrLayerShellHandler, WlrLayerShellState,
};
use smithay::wayland::shell::xdg::PopupSurface;

/// The key of the output a client's `wl_output` names.
pub(super) fn output_key_of(state: &State, wl_output: &WlOutput) -> Option<OutputKey> {
    let named = Output::from_resource(wl_output)?;
    state
        .wl
        .output
        .outputs
        .iter()
        .find(|(_, output)| **output == named)
        .map(|(key, _)| key.clone())
}

/// The output a layer lands on when its client names none: the leftmost, then topmost, plugged
/// output (the model's own primary rule, which it does not export).
fn primary_output(state: &State) -> Option<OutputKey> {
    state
        .core
        .model
        .outputs()
        .0
        .iter()
        .min_by_key(|(key, output)| {
            let at = output.config.position;
            (at.x, at.y, (*key).clone())
        })
        .map(|(key, _)| key.clone())
}

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
        let Some(output) = output
            .as_ref()
            .and_then(|wl_output| output_key_of(self, wl_output))
            .or_else(|| primary_output(self))
        else {
            // No output to put it on: the protocol's answer is `closed`.
            surface.send_close();
            return;
        };
        let (mint, key) = self.core.mint.clone().layer();
        self.core.mint = mint;
        let events = layer_shell::on_new_layer(
            &mut self.wl.layer_shell,
            &surface,
            Some(output),
            layer_shell::level_of(layer),
            LayerNamespace(namespace),
            key,
        );
        core_loop::feed(self, events);
    }

    fn new_popup(&mut self, parent: LayerSurface, popup: PopupSurface) {
        let events = layer_shell::on_layer_popup(&mut self.wl.layer_shell, &parent, &popup);
        let popups: &mut PopupManager = &mut self.wl.xdg_shell.popups;
        if let Err(error) = popups.track_popup(PopupKind::Xdg(popup.clone())) {
            tracing::warn!(%error, "a layer popup could not be tracked");
        }
        if let Err(error) = popup.send_configure() {
            tracing::warn!(%error, "a layer popup could not be configured");
        }
        core_loop::feed(self, events);
    }

    fn layer_destroyed(&mut self, surface: LayerSurface) {
        let events = layer_shell::on_layer_destroyed(&mut self.wl.layer_shell, &surface);
        core_loop::feed(self, events);
    }
}
