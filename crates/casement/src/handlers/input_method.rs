//! `zwp_input_method_manager_v2` (P0-H7): the candidate popup callbacks, and the consent gate
//! (`Power::TypingHelp`: an input method sees what a person types, so smithay asks
//! `input_method_gate` when a client asks to be the input method and settles it when the answer
//! comes).

use crate::core_loop;
use crate::state::State;
use casement_core::Event;
use casement_protocols::vocab::Power;
use casement_wayland::convert::rect_to;
use casement_wayland::{consent, input_method, text_input};
use smithay::reexports::wayland_server::Client;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Rectangle};
use smithay::wayland::Gate;
use smithay::wayland::input_method::{InputMethodHandler, PopupSurface};

/// Feeds `events` to the model when there are any (the popup callbacks mostly have none).
fn feed_any(state: &mut State, events: Vec<Event>) {
    if !events.is_empty() {
        core_loop::feed(state, events);
    }
}

impl InputMethodHandler for State {
    fn input_method_gate(&mut self, client: &Client) -> Gate {
        consent::gate(self, client, Power::TypingHelp)
    }

    /// smithay calls this when the input method activates on a field (and when a popup is
    /// created over one): the popup is kept for the scene, and a new activation opens for the
    /// key gate.
    fn new_popup(&mut self, surface: PopupSurface) {
        let mut events = input_method::on_im_popup(&mut self.wl.input_method, &surface);
        if let Some(parent) = surface.get_parent() {
            events.extend(text_input::on_enable(
                &mut self.wl.text_input,
                &parent.surface,
            ));
        }
        feed_any(self, events);
    }

    fn dismiss_popup(&mut self, surface: PopupSurface) {
        let events = input_method::on_im_popup_dismissed(&mut self.wl.input_method, &surface);
        feed_any(self, events);
    }

    /// The field moved its cursor rectangle: the popup follows.
    fn popup_repositioned(&mut self, surface: PopupSurface) {
        let mut events = input_method::on_im_popup(&mut self.wl.input_method, &surface);
        if let Some(parent) = surface.get_parent() {
            events.extend(text_input::on_commit_state(
                &mut self.wl.text_input,
                &parent.surface,
            ));
        }
        feed_any(self, events);
    }

    /// Where the parent window is, in global logical coordinates: its arranged rectangle. A
    /// parent that is not a toplevel (a layer surface) answers an empty rectangle at the
    /// origin.
    fn parent_geometry(&self, parent: &WlSurface) -> Rectangle<i32, Logical> {
        self.wl
            .xdg_shell
            .toplevels
            .iter()
            .find(|(_, toplevel)| toplevel.wl_surface() == parent)
            .and_then(|(window, _)| {
                self.core
                    .arrangement
                    .windows
                    .iter()
                    .find(|target| target.window == *window)
            })
            .map_or_else(Rectangle::default, |target| rect_to(target.rect))
    }
}
