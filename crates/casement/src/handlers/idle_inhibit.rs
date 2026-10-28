//! `zwp_idle_inhibit_manager_v1` (P0-H8): inhibitors change what the idle machine counts, so
//! each change is read back through `core_loop::sync_inhibitors`, which counts only the ones
//! the person can see.

use crate::core_loop::sync_inhibitors;
use crate::state::State;
use casement_wayland::idle_inhibit::{on_inhibit, on_uninhibit};
use casement_wayland::limits::{self, Limit};
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::idle_inhibit::IdleInhibitHandler;

impl IdleInhibitHandler for State {
    fn inhibit(&mut self, surface: WlSurface) {
        // A client past its cap is cut after this request (NEW-CAS-2) and its inhibitor not counted.
        if let Some(client) = surface.client() {
            let held = self.wl.idle_inhibit.held_by(&client);
            if !limits::admit(&self.display, &client, Limit::IdleInhibitors, held) {
                return;
            }
        }
        on_inhibit(&mut self.wl.idle_inhibit, &surface);
        sync_inhibitors(self);
    }

    fn uninhibit(&mut self, surface: WlSurface) {
        on_uninhibit(&mut self.wl.idle_inhibit, &surface);
        sync_inhibitors(self);
    }
}
