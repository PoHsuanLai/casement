//! `wl_compositor` (P0-H1): every commit enters here and is routed by its surface's role.

use crate::core_loop;
use crate::state::State;
use casement_core::Event;
use casement_wayland::compositor::{self, CommitRoute};
use casement_wayland::{
    ClientState, commit_timing, content_type, fifo, fractional_scale, layer_shell, presentation,
    subcompositor, xdg_shell,
};
use smithay::backend::renderer::utils::on_commit_buffer_handler;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, Resource};
use smithay::wayland::compositor::{
    CompositorClientState, CompositorHandler, CompositorState, add_pre_commit_hook,
};

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.wl.compositor.state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        match ClientState::of(client) {
            Some(state) => &state.compositor,
            // Every client is inserted through `clients::insert_client` with a ClientState;
            // a client without one cannot exist, and smithay's signature leaves no error path.
            None => unreachable!("a client inserted without casement's ClientState"),
        }
    }

    /// Every surface starts with the primary output's preferred scale, so a client that
    /// creates its `wp_fractional_scale_v1` at any point learns it with the object, before the
    /// surface's first configure; and watches for commit timers (smithay holds timed commits
    /// back before `commit` runs, so the barrier is seen in a pre-commit hook).
    fn new_surface(&mut self, surface: &WlSurface) {
        add_pre_commit_hook::<State, _>(surface, |state, _display, surface| {
            let events = commit_timing::on_barrier(&mut state.wl.commit_timing, surface);
            feed(state, events);
        });
        if let Some(scale) = fractional_scale::home_scale(self.core.model.outputs())
            && fractional_scale::apply_scale(&mut self.wl.fractional_scale, surface, scale).is_err()
        {
            tracing::debug!("a surface was destroyed before it got its preferred scale");
        }
    }

    fn new_subsurface(&mut self, surface: &WlSurface, parent: &WlSurface) {
        let events = subcompositor::on_new_subsurface(&mut self.wl.subcompositor, surface, parent);
        feed(self, events);
    }

    fn commit(&mut self, surface: &WlSurface) {
        // Buffer bookkeeping every role shares; it also validates viewports (`out_of_buffer`).
        on_commit_buffer_handler::<State>(surface);
        let route = compositor::on_commit(&mut self.wl.compositor, surface);
        let mut events = presentation::on_feedback(&mut self.wl.presentation, surface);
        events.extend(fifo::on_barrier(&mut self.wl.fifo, surface));
        events.extend(content_type::on_content_type(
            &mut self.wl.content_type,
            surface,
            content_type::committed_kind(surface),
        ));
        events.extend(self.commit_role(route, surface));
        feed(self, events);
        core_loop::refocus_pointer(self);
    }

    fn destroyed(&mut self, surface: &WlSurface) {
        presentation::discard_surface(&mut self.wl.presentation, surface);
        fifo::forget_surface(&mut self.wl.fifo, surface);
        commit_timing::forget_surface(&mut self.wl.commit_timing, surface);
        let events = compositor::on_destroyed(&mut self.wl.compositor, surface);
        feed(self, events);
    }
}

impl State {
    /// What a commit means to the module that owns the surface's role.
    fn commit_role(&mut self, route: CommitRoute, surface: &WlSurface) -> Vec<Event> {
        match route {
            CommitRoute::Toplevel => {
                let Some(client) = surface.client() else {
                    return Vec::new();
                };
                match ClientState::of(&client) {
                    Some(state) => {
                        xdg_shell::on_toplevel_commit(&mut self.wl.xdg_shell, surface, state)
                    }
                    None => Vec::new(),
                }
            }
            CommitRoute::Popup => xdg_shell::on_popup_commit(&mut self.wl.xdg_shell, surface),
            CommitRoute::Layer => layer_shell::on_layer_commit(&mut self.wl.layer_shell, surface),
            CommitRoute::LockSurface
            | CommitRoute::Subsurface
            | CommitRoute::Cursor
            | CommitRoute::Unassigned => Vec::new(),
        }
    }

    /// Lets the commits that fifo barriers and commit timers held back go: the presenting side
    /// (`core_loop::frame`) calls this after `fifo::apply_presented` and
    /// `commit_timing::apply_presented`, which only signal the barriers; smithay applies a
    /// held commit when its client's compositor state is told the blocker cleared.
    pub fn release_blockers(&mut self) {
        let clients: Vec<Client> = fifo::take_cleared(&mut self.wl.fifo)
            .into_iter()
            .chain(commit_timing::take_cleared(&mut self.wl.commit_timing))
            .collect();
        let display = self.display.clone();
        for client in clients {
            self.client_compositor_state(&client)
                .blocker_cleared(self, &display);
        }
    }
}

/// Steps `events` through the model (nothing to do for none).
fn feed(state: &mut State, events: Vec<Event>) {
    if !events.is_empty() {
        core_loop::feed(state, events);
    }
}
