//! `zxdg_decoration_manager_v1` (P0-H2): client-side preferred; every wish is answered with
//! `xdg_decoration::mode_for`.

use crate::core_loop;
use crate::state::State;
use casement_wayland::xdg_decoration::{
    DecorationWish, apply_mode, mode_for, on_decoration_mode,
};
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
use smithay::wayland::shell::xdg::ToplevelSurface;
use smithay::wayland::shell::xdg::decoration::XdgDecorationHandler;

impl State {
    /// `toplevel` expressed `wish`: the model hears of it (nothing, at P0) and the client is
    /// told who draws its frame.
    fn decorate(&mut self, toplevel: &ToplevelSurface, wish: DecorationWish) {
        let events = on_decoration_mode(&mut self.wl.xdg_decoration, toplevel, wish);
        core_loop::feed(self, events);
        if let Err(error) = apply_mode(&mut self.wl.xdg_decoration, toplevel, mode_for(wish)) {
            tracing::debug!(%error, "a decoration answer reached a toplevel that is gone");
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
