//! `zwlr_layer_shell_v1` (P0-H3).

use super::xdg_shell::popups;
use crate::core_loop;
use crate::state::State;
use casement_core::OutputKey;
use casement_core::layer::LayerOwner;
use casement_core::policy::ConnectionRole;
use casement_wayland::ClientState;
use casement_wayland::layer_shell::{self, LayerNamespace};
use casement_wayland::limits::{self, Limit, Standing};
use smithay::output::Output;
use smithay::reexports::wayland_server::Resource;
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

/// Whose connection `surface` came through: the shell's own, or another client's (a surface
/// whose client is gone counts as another client's).
fn owner_of(surface: &LayerSurface) -> LayerOwner {
    let role = surface
        .wl_surface()
        .client()
        .and_then(|client| ClientState::of(&client).map(|state| state.role.clone()));
    match role {
        Some(ConnectionRole::Shell) => LayerOwner::Shell,
        Some(_) | None => LayerOwner::Client,
    }
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
        // A client past its cap is cut (CASC-03); no layer is made for the one it asked for.
        if let Some(client) = surface.wl_surface().client() {
            let held =
                ClientState::of(&client).map_or(0, |data| self.wl.layer_shell.held_by(data.key));
            if limits::admit(&self.display, &client, Limit::Layers, held) == Standing::Over {
                return;
            }
        }
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
            owner_of(&surface),
            key,
        );
        core_loop::feed(self, events);
    }

    fn new_popup(&mut self, parent: LayerSurface, popup: PopupSurface) {
        let events = layer_shell::on_layer_popup(&mut self.wl.layer_shell, &parent, &popup);
        // Placed by its positioner inside the output, as the popups of windows are.
        let positioner = popup.with_pending_state(|state| state.positioner);
        popups::new_popup(self, popup.clone(), positioner);
        if let Err(error) = popup.send_configure() {
            tracing::warn!(%error, "a layer popup could not be configured");
        }
        core_loop::feed(self, events);
    }

    fn layer_destroyed(&mut self, surface: LayerSurface) {
        if let Some(key) = self.wl.layer_shell.key_of(surface.wl_surface()) {
            casement_wayland::quire::shell::on_layer_gone(&mut self.wl.quire_shell, key);
        }
        let events = layer_shell::on_layer_destroyed(&mut self.wl.layer_shell, &surface);
        core_loop::feed(self, events);
    }
}
