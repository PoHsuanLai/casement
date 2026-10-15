//! `zxdg_decoration_manager_v1` (P0-H2): the compositor draws the frame unless the client
//! insists on its own; every wish is answered with `xdg_decoration::mode_for`.

use crate::core_loop;
use crate::exec;
use crate::state::State;
use casement_core::{Cause, Effect};
use casement_wayland::xdg_decoration::{
    DecorationMode, DecorationWish, apply_mode, mode_for, on_decoration_mode,
};
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
use smithay::wayland::shell::xdg::ToplevelSurface;
use smithay::wayland::shell::xdg::decoration::XdgDecorationHandler;

impl State {
    /// `toplevel` expressed `wish`: the model hears of it (nothing) and the client is
    /// told who draws its frame.
    fn decorate(&mut self, toplevel: &ToplevelSurface, wish: DecorationWish) {
        let events = on_decoration_mode(&mut self.wl.xdg_decoration, toplevel, wish);
        core_loop::feed(self, events);
        let mode = mode_for(wish);
        if let Some(window) = self.wl.xdg_shell.window_of_surface(toplevel.wl_surface()) {
            self.wl.xdg_decoration.record(window, mode);
            // A server-side frame reserves its titlebar in layout: a window the model already
            // placed is laid out again with it.
            if mode == DecorationMode::ServerSide && self.core.model.windows().contains_key(&window)
            {
                exec::exec(self, vec![Effect::Relayout(Cause::Configure)]);
            }
        }
        if let Err(error) = apply_mode(&mut self.wl.xdg_decoration, toplevel, mode) {
            tracing::debug!(%error, "a decoration answer reached a toplevel that is gone");
        }
        // A37: the decoration's configure is its first word and rides on an xdg_surface
        // configure; wlcs' decoration tests wait for it without ever committing, so under the
        // lenient policy the initial configure goes out with it (`testing` builds only).
        #[cfg(feature = "testing")]
        if self.ack_policy == crate::relax::AckPolicy::Lenient
            && toplevel.alive()
            && !toplevel.is_initial_configure_sent()
        {
            toplevel.send_configure();
        }
    }
}

impl XdgDecorationHandler for State {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        self.decorate(&toplevel, DecorationWish::Unset);
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: Mode) {
        let wish = match mode {
            Mode::ClientSide => DecorationWish::ClientSide,
            Mode::ServerSide => DecorationWish::ServerSide,
            _ => DecorationWish::Unset,
        };
        self.decorate(&toplevel, wish);
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        self.decorate(&toplevel, DecorationWish::Unset);
    }
}
