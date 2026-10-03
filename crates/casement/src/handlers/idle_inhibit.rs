//! `zwp_idle_inhibit_manager_v1` (P0-H8): inhibitors change what the idle machine counts, so
//! each change goes to `casement_wayland::idle_notify::on_inhibit` with the session's now.

use crate::exec::exec;
use crate::state::State;
use casement_wayland::idle_inhibit::{on_inhibit, on_uninhibit};
use casement_wayland::idle_notify::{self, Inhibited};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::idle_inhibit::IdleInhibitHandler;

impl IdleInhibitHandler for State {
    fn inhibit(&mut self, surface: WlSurface) {
        let inhibited = on_inhibit(&mut self.wl.idle_inhibit, &surface);
        sync(self, inhibited);
    }

    fn uninhibit(&mut self, surface: WlSurface) {
        let inhibited = on_uninhibit(&mut self.wl.idle_inhibit, &surface);
        sync(self, inhibited);
    }
}

/// Tells the idle machine what the inhibitors now say, at the session's now.
fn sync(state: &mut State, inhibited: Inhibited) {
    let now = state.clock.now();
    let effects = idle_notify::on_inhibit(&mut state.wl.idle_notify, inhibited, now);
    exec(state, effects);
}
