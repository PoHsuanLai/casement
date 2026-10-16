//! `quire_shell_v1` (P2-Z; filled by P2-Q2): the wire glue lives in
//! `casement_wayland::quire::shell`; this implements what it asks of the state.

use crate::core_loop;
use crate::state::State;
use casement_core::time::Mono;
use casement_core::{Event, LayerKey, OutputKey, WindowId};
use casement_wayland::quire::shell::{QuireShellHandler, QuireShellPart};
use smithay::output::Output;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, Resource};

impl QuireShellHandler for State {
    fn quire_shell_part(&mut self) -> &mut QuireShellPart {
        &mut self.wl.quire_shell
    }

    fn quire_shell_now(&self) -> Mono {
        self.clock.now()
    }

    fn quire_shell_events(&mut self, events: Vec<Event>) {
        core_loop::feed(self, events);
    }

    fn layer_of_surface(&self, client: &Client, surface: &WlSurface) -> Option<LayerKey> {
        let key = self.wl.layer_shell.key_of(surface)?;
        (surface.client().as_ref() == Some(client)).then_some(key)
    }

    fn window_known(&self, window: WindowId) -> bool {
        self.core.model.windows().contains_key(&window)
    }

    fn output_key_of(&self, output: &WlOutput) -> Option<OutputKey> {
        let advertised = Output::from_resource(output)?;
        self.wl
            .output
            .outputs
            .iter()
            .find(|(_, held)| **held == advertised)
            .map(|(key, _)| key.clone())
    }

    fn space_known(&self, space: &porter_core::SpaceId) -> bool {
        self.core.model.spaces().get(space).is_some()
    }
}
