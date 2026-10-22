//! `quire_shell_v1` (P2-Z; filled by P2-Q2): the wire glue lives in
//! `casement_wayland::quire::shell`; this implements what it asks of the state.

use crate::state::State;
use casement_core::privilege::UnixSeconds;
use casement_core::time::Mono;
use casement_core::{LayerKey, OutputKey, WindowId};
use casement_protocols::vocab::TrialId;
use casement_wayland::quire::shell::{QuireShellHandler, QuireShellPart};
use casement_wayland::quire::shell_trial;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, Resource};
use std::time::Duration;

impl QuireShellHandler for State {
    fn quire_shell_part(&mut self) -> &mut QuireShellPart {
        &mut self.wl.quire_shell
    }

    fn quire_shell_now(&self) -> Mono {
        self.clock.now()
    }

    fn quire_shell_wall(&self) -> UnixSeconds {
        self.seams.wall.now()
    }

    fn layer_of_surface(&self, client: &Client, surface: &WlSurface) -> Option<LayerKey> {
        let key = self.wl.layer_shell.key_of(surface)?;
        (surface.client().as_ref() == Some(client)).then_some(key)
    }

    fn layer_output(&self, layer: LayerKey) -> Option<OutputKey> {
        self.wl
            .layer_shell
            .records
            .get(&layer)
            .and_then(|record| record.output.clone())
    }

    fn window_known(&self, window: WindowId) -> bool {
        self.core.model.windows().contains_key(&window)
    }

    fn space_known(&self, space: &porter_core::SpaceId) -> bool {
        self.core.model.spaces().get(space).is_some()
    }
}

/// `Effect::AskKeepDisplays`: the question about the changed displays goes to the shell.
pub(crate) fn trial_ask(state: &mut State, trial: TrialId, within: Duration) {
    let now = state.clock.now();
    shell_trial::apply_trial(&mut state.wl.quire_shell, trial, within, now);
}

/// `Effect::EndDisplaysTrial`: the question about the displays is taken down.
pub(crate) fn trial_end(state: &mut State, trial: TrialId) {
    shell_trial::apply_end(&mut state.wl.quire_shell, trial);
}
