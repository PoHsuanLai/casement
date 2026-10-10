//! `quire_shell_v1` (P2-Z; filled by P2-Q2): the wire glue lives in
//! `casement_wayland::quire::shell`; this implements what it asks of the state.

use crate::core_loop;
use crate::state::State;
use casement_core::{Event, LayerKey};
use casement_wayland::quire::shell::{QuireShellHandler, QuireShellPart};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, Resource};

impl QuireShellHandler for State {
    fn quire_shell_part(&mut self) -> &mut QuireShellPart {
        &mut self.wl.quire_shell
    }

    fn quire_shell_events(&mut self, events: Vec<Event>) {
        core_loop::feed(self, events);
    }

    fn layer_of_surface(&self, client: &Client, surface: &WlSurface) -> Option<LayerKey> {
        let key = self.wl.layer_shell.key_of(surface)?;
        (surface.client().as_ref() == Some(client)).then_some(key)
    }
}
