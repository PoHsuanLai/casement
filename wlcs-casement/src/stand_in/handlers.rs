//! smithay handler impls for the stand-in: compositor commit, shm, xdg-shell, seat, selection.

use super::StandIn;
use smithay::backend::renderer::utils::on_commit_buffer_handler;
use smithay::desktop::{PopupKind, Window};
use smithay::input::dnd::DndGrabHandler;
use smithay::input::pointer::CursorImageStatus;
use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::{
    wl_buffer, wl_output::WlOutput, wl_seat, wl_surface::WlSurface,
};
use smithay::reexports::wayland_server::{Client, Resource};
use smithay::utils::Serial;
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::compositor::{
    CompositorClientState, CompositorHandler, CompositorState, get_parent, is_sync_subsurface,
    with_states,
};
use smithay::wayland::output::OutputHandler;
use smithay::wayland::seat::WaylandFocus;
use smithay::wayland::selection::SelectionHandler;
use smithay::wayland::selection::data_device::{
    DataDeviceHandler, DataDeviceState, WaylandDndGrabHandler, set_data_device_focus,
};
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
    XdgToplevelSurfaceData,
};
use smithay::wayland::shm::{ShmHandler, ShmState};
use std::sync::OnceLock;

/// What every client of the stand-in carries.
#[derive(Debug, Default)]
pub(super) struct ClientState {
    compositor: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _: ClientId) {}
    fn disconnected(&self, _: ClientId, _: DisconnectReason) {}
}

/// What a client without our `ClientState` would get; every client is inserted with one.
fn fallback_client_state() -> &'static CompositorClientState {
    static FALLBACK: OnceLock<CompositorClientState> = OnceLock::new();
    FALLBACK.get_or_init(CompositorClientState::default)
}

impl CompositorHandler for StandIn {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        match client.get_data::<ClientState>() {
            Some(state) => &state.compositor,
            None => fallback_client_state(),
        }
    }

    fn new_surface(&mut self, surface: &WlSurface) {
        #[cfg(feature = "testing")]
        super::relax::install(surface);
        #[cfg(not(feature = "testing"))]
        let _ = surface;
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self.window_for(&root) {
                window.on_commit();
            }
        }
        if let Some(window) = self.window_for(surface) {
            let sent = with_states(surface, |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .is_some_and(|d| d.lock().is_ok_and(|d| d.initial_configure_sent))
            });
            if !sent && let Some(toplevel) = window.toplevel() {
                toplevel.send_configure();
            }
        }
        self.commit_layer(surface);
        self.popups.commit(surface);
        if let Some(PopupKind::Xdg(popup)) = self.popups.find_popup(surface)
            && !popup.is_initial_configure_sent()
        {
            let _ = popup.send_configure();
        }
    }
}

/// Whether a state is being set or unset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Apply {
    Set,
    Unset,
}

impl StandIn {
    /// Sets or unsets `state` and sizes the toplevel to the output (or to its own choice); the
    /// protocol demands a configure either way once the initial one is out.
    fn resize_to_output(
        &mut self,
        surface: &ToplevelSurface,
        state: xdg_toplevel::State,
        apply: Apply,
    ) {
        let size = self
            .space
            .outputs()
            .next()
            .and_then(|o| self.space.output_geometry(o))
            .map(|g| g.size);
        surface.with_pending_state(|pending| match apply {
            Apply::Set => {
                pending.states.set(state);
                pending.size = size;
            }
            Apply::Unset => {
                pending.states.unset(state);
                pending.size = None;
            }
        });
        if surface.is_initial_configure_sent() {
            surface.send_configure();
        }
    }

    fn window_for(&self, surface: &WlSurface) -> Option<Window> {
        self.space
            .elements()
            .find(|w| w.wl_surface().is_some_and(|s| *s == *surface))
            .cloned()
    }
}

impl BufferHandler for StandIn {
    fn buffer_destroyed(&mut self, _: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for StandIn {
    fn shm_state(&self) -> &ShmState {
        &self.shm
    }
}

impl XdgShellHandler for StandIn {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        self.space
            .map_element(Window::new_wayland_window(surface), (0, 0), true);
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        surface.with_pending_state(|s| s.geometry = s.positioner.get_geometry());
        let _ = self.popups.track_popup(PopupKind::Xdg(surface));
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        surface.with_pending_state(|s| {
            s.geometry = positioner.get_geometry();
            s.positioner = positioner;
        });
        surface.send_repositioned(token);
    }

    fn fullscreen_request(&mut self, surface: ToplevelSurface, _output: Option<WlOutput>) {
        self.resize_to_output(&surface, xdg_toplevel::State::Fullscreen, Apply::Set);
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        self.resize_to_output(&surface, xdg_toplevel::State::Fullscreen, Apply::Unset);
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        self.resize_to_output(&surface, xdg_toplevel::State::Maximized, Apply::Set);
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        self.resize_to_output(&surface, xdg_toplevel::State::Maximized, Apply::Unset);
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {}
}

impl SeatHandler for StandIn {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<StandIn> {
        &mut self.seat_state
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {}

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        let client = focused.and_then(|s| self.dh.get_client(s.id()).ok());
        set_data_device_focus(&self.dh, seat, client);
    }
}

impl SelectionHandler for StandIn {
    type SelectionUserData = ();
}

impl DataDeviceHandler for StandIn {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.data_device
    }
}

impl DndGrabHandler for StandIn {}
impl WaylandDndGrabHandler for StandIn {}
impl OutputHandler for StandIn {}

smithay::delegate_dispatch2!(StandIn);
