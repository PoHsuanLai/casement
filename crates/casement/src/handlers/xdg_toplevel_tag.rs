//! `xdg_toplevel_tag_manager_v1` (P0-H2): the tag is kept for placement memory (P3).

use crate::core_loop;
use crate::state::State;
use casement_wayland::xdg_toplevel_tag::{ToplevelTag, on_tag};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::XdgToplevel;
use smithay::wayland::xdg_toplevel_tag::XdgToplevelTagHandler;

impl XdgToplevelTagHandler for State {
    fn set_tag(&mut self, toplevel: XdgToplevel, tag: String) {
        let Some(window) = self.wl.xdg_shell.window_of_toplevel(&toplevel) else {
            return;
        };
        let events = on_tag(&mut self.wl.xdg_toplevel_tag, window, ToplevelTag(tag));
        core_loop::feed(self, events);
    }
}
