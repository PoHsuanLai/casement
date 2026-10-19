//! The fixture for a client the test does not drive: a program in another thread (wlcs) that
//! owns its end of a socket pair and talks at its own pace. Such a client needs the server end
//! adopted ([`Fixture::adopt_socket`]), a way to wait for it to speak ([`Fixture::wait`]) and
//! the two things the model owns that a client cannot reach: which window a surface is
//! ([`Fixture::toplevel_of`]) and where it sits ([`Fixture::place_window`]).

use super::{ClientId, Fixture, arriving_standing};
use casement::core_loop;
use casement_core::ToplevelRequest;
use casement_core::focus::TokenCheck;
use casement_core::geometry::{LogicalPoint, LogicalRect};
use casement_core::policy::ConnectionRole;
use casement_core::{Event, LayerKey, WindowId};
use casement_layout::{LayoutKnobs, RustScene, SceneLayout, frame_metrics, spec_of_framed};
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use std::collections::BTreeMap;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::time::Duration;

impl Fixture {
    /// A new client of `role` whose end of the socket pair is handed to the caller, who drives
    /// it from elsewhere: no [`TestClient`](casement_testclient::TestClient) is made, so the
    /// fixture's pump has nothing to dispatch for it. The fixture pumps once so the compositor
    /// has seen it arrive. It arrives with the standing [`Fixture::connect`] gives a client of
    /// its role (a public one is a desktop service the table trusts), so what it uses is not
    /// asked about: a driven client such as wlcs tests the protocols, not the consent flow.
    pub fn adopt_socket(&mut self, role: ConnectionRole) -> (ClientId, OwnedFd) {
        let (server, client) = UnixStream::pair().expect("a socket pair for the adopted client");
        server
            .set_nonblocking(true)
            .expect("a non-blocking server end");
        let standing = arriving_standing(&role);
        let held = self.insert(server, role, standing, None);
        let id = ClientId(self.next_client);
        self.servers.insert(id, held);
        self.next_client += 1;
        self.pump();
        (id, OwnedFd::from(client))
    }

    /// Blocks up to `timeout` of real time for the compositor's event loop to have something
    /// to do (a client wrote), and does it. The virtual clock does not move: the caller
    /// advances it.
    pub fn wait(&mut self, timeout: Duration) {
        self.event_loop
            .dispatch(Some(timeout), &mut self.state)
            .expect("the event loop dispatches");
    }

    /// The toplevel that surface object `object` of `client` is the root of, if it is one.
    pub fn toplevel_of(&self, client: ClientId, object: u32) -> Option<WindowId> {
        let held = self.servers.get(&client)?;
        let surface: WlSurface = held
            .object_from_protocol_id(&self.state.display, object)
            .ok()?;
        self.state
            .wl
            .xdg_shell
            .toplevels
            .iter()
            .find(|(_, toplevel)| toplevel.wl_surface().id() == surface.id())
            .map(|(window, _)| *window)
    }

    /// The layer surface that surface object `object` of `client` is, if it is one.
    pub fn layer_of(&self, client: ClientId, object: u32) -> Option<LayerKey> {
        let held = self.servers.get(&client)?;
        let surface: WlSurface = held
            .object_from_protocol_id(&self.state.display, object)
            .ok()?;
        self.state.wl.layer_shell.key_of(&surface)
    }

    /// Feeds `events` to the core as if the protocol layer had produced them (for a protocol
    /// whose client the kit does not have, such as the global menu ones); then pumps.
    pub fn feed_events(&mut self, events: Vec<Event>) {
        core_loop::feed(&mut self.state, events);
        self.pump();
    }

    /// Puts `layer`'s top-left corner at `at` wherever its anchors would have put it
    /// ([`Event::LayerPlaced`]); then pumps.
    pub fn place_layer(&mut self, layer: LayerKey, at: LogicalPoint) {
        core_loop::feed(&mut self.state, vec![Event::LayerPlaced { layer, at }]);
        self.pump();
    }

    /// Puts `window`'s top-left corner at `at`, keeping its size, as if the person had
    /// dragged it there (`Event::WindowDragged`); then pumps. A window the layout does not
    /// hold is left alone.
    pub fn place_window(&mut self, window: WindowId, at: LogicalPoint) {
        let held = self
            .state
            .core
            .arrangement
            .windows
            .iter()
            .find(|target| target.window == window)
            .map(|target| target.rect.size);
        if let Some(size) = held {
            let rect = LogicalRect::new(at, size);
            core_loop::feed(&mut self.state, vec![Event::WindowDragged { window, rect }]);
        }
        self.pump();
    }

    /// Puts `window`'s top-left corner at `at`, keeping its size, where it is told even if that
    /// is partly outside the usable area (`Event::WindowPlaced`: a test's placement, not the
    /// person's drag, which keeps a window within reach); then pumps. A window the layout does not
    /// hold is left alone.
    pub fn place_window_by_hand(&mut self, window: WindowId, at: LogicalPoint) {
        let held = self
            .state
            .core
            .arrangement
            .windows
            .iter()
            .find(|target| target.window == window)
            .map(|target| target.rect.size);
        if let Some(size) = held {
            let rect = LogicalRect::new(at, size);
            core_loop::feed(&mut self.state, vec![Event::WindowPlaced { window, rect }]);
        }
        self.pump();
    }

    /// Activates `window` as a person would with a valid token (it is keyed, shown and
    /// configured activated); then pumps.
    pub fn activate_window(&mut self, window: WindowId) {
        let request = Event::ToplevelRequest {
            window,
            request: ToplevelRequest::Activate(TokenCheck::Valid),
        };
        core_loop::feed(&mut self.state, vec![request]);
        self.pump();
    }

    /// The surface the seat's pointer is on now, as the compositor holds it (none when the
    /// pointer is over nothing a client owns).
    pub fn pointer_focus(&self) -> Option<WlSurface> {
        self.state
            .wl
            .seat
            .pointer
            .current_focus()
            .and_then(Into::into)
    }

    /// Hands the pointer to whatever is under it now (a window that glided to its place under
    /// a still pointer has moved what is there); then pumps.
    pub fn refocus_pointer(&mut self) {
        core_loop::refocus_pointer(&mut self.state);
        self.pump();
    }

    /// Paints the server-side frames in `scheme`, as the desktop's colour scheme does once
    /// `appearance.theme = "system"` has resolved against it (that table is
    /// `casement::appearance::resolve`'s, unit tested there; the watch on the settings portal
    /// and the file is a thread, which a virtual-clock fixture does not drive); then pumps.
    pub fn set_scheme(&mut self, scheme: casement_layout::Scheme) {
        self.state.render.scheme = scheme;
        self.pump();
    }

    /// Where the layout puts `window`'s content when casement frames it (the server-side
    /// decoration): the titlebar is reserved above it. The window must be arranged.
    pub fn framed_content_of(&self, window: WindowId) -> LogicalRect {
        let model = self.model();
        let frames = BTreeMap::from([(window, frame_metrics().titlebar)]);
        RustScene
            .arrange(
                &spec_of_framed(model, &frames),
                &LayoutKnobs::of(model.settings()),
            )
            .windows
            .iter()
            .find(|target| target.window == window)
            .map(|target| target.rect)
            .expect("the window is arranged")
    }
}
