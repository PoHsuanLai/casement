//! `xdg_wm_base` (P0-H2): each smithay callback converts its arguments, asks
//! `casement_wayland::xdg_shell` for events and feeds them to the model. The model decides
//! (where a window goes, whether a press focuses it, what a configure says); a move or resize
//! only goes on after the seat says its serial answers a real press.

mod menu_grab;
pub(super) mod popups;

use crate::core_loop;
use crate::state::State;
use casement_core::{ClientKey, InputSerial, OutputKey, WindowId};
use casement_protocols::vocab::Toggle;
use casement_wayland::convert::{point_of, serial_of};
use casement_wayland::limits::{self, Limit, Standing};
use casement_wayland::serial::PressRecord;
use casement_wayland::xdg_shell::{
    self, GrabUse, edges_of, grab_proven, heaviest_holder, on_app_id, on_full_screen, on_maximise,
    on_minimise, on_move, on_new_toplevel, on_parent, on_pong, on_resize, on_title,
    on_toplevel_destroyed, on_window_menu,
};
use casement_wayland::{ClientState, xdg_foreign, xdg_toplevel_tag};
use smithay::desktop::{PopupKind, find_popup_root_surface};
use smithay::output::Output;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, Resource};
use smithay::utils::{Logical, Point, Serial};
use smithay::wayland::shell::xdg::{
    Configure, PopupSurface, PositionerState, ShellClient, ToplevelSurface, XdgShellHandler,
    XdgShellState,
};

/// Who is cut when the session is out of room for a window.
enum Cut {
    /// The client that asked.
    Asker,
    /// Another client, the one holding the most windows.
    Other(Client),
}

impl State {
    /// The next window id (minted here, where the toplevel is first seen).
    fn mint_window(&mut self) -> WindowId {
        let mint = std::mem::replace(&mut self.core.mint, casement_core::IdMint::start());
        let (mint, window) = mint.window();
        self.core.mint = mint;
        window
    }

    /// The press the seat recorded for `serial`, if it was delivered to `client`.
    pub(crate) fn press_of(&self, client: &Client, serial: InputSerial) -> Option<PressRecord> {
        let key = ClientState::of(client)?.key;
        self.wl.seat.serials.answers(serial, key).copied()
    }

    /// Whether `serial`, sent by the owner of `surface`, answers a press that `use_` accepts.
    fn serial_proves(&self, surface: &WlSurface, serial: InputSerial, use_: GrabUse) -> bool {
        surface
            .client()
            .is_some_and(|client| grab_proven(self.press_of(&client, serial).as_ref(), use_))
    }

    /// Whether `serial` is the latest input its client's seat delivered to it and a popup grab
    /// may answer it (a press, a release, a key or a touch down).
    pub(crate) fn grab_serial_proves(&self, surface: &WlSurface, serial: InputSerial) -> bool {
        let Some(key) = surface
            .client()
            .and_then(|client| ClientState::of(&client).map(|data| data.key))
        else {
            return false;
        };
        let record = self.wl.seat.serials.answers_latest(serial, key);
        grab_proven(record, GrabUse::AnyPress)
    }

    /// Who is cut because the session holds as many windows as it may and `asker` wants one more:
    /// the client holding the most, or `asker` when it is that client (or cannot be told apart).
    fn cut_for_room(&self, asker: &Client) -> Cut {
        let Some(key) = ClientState::of(asker).map(|data| data.key) else {
            return Cut::Asker;
        };
        let held = self.wl.xdg_shell.toplevels_by_client();
        match heaviest_holder(&held, key) {
            heaviest if heaviest == key => Cut::Asker,
            heaviest => self
                .wl
                .xdg_shell
                .connection_of(heaviest)
                .map_or(Cut::Asker, Cut::Other),
        }
    }

    /// The output a client's `wl_output` is, as the model keys it.
    fn output_key_by_name(&self, output: &WlOutput) -> Option<OutputKey> {
        let name = Output::from_resource(output)?.name();
        self.core
            .model
            .outputs()
            .0
            .iter()
            .find(|(_, state)| state.facts.name.as_str() == name)
            .map(|(key, _)| key.clone())
    }
}

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.wl.xdg_shell.state
    }

    fn client_pong(&mut self, client: ShellClient) {
        let keys: Vec<ClientKey> = self
            .wl
            .xdg_shell
            .clients
            .iter()
            .filter(|(_, held)| **held == client)
            .map(|(key, _)| *key)
            .collect();
        let events = keys
            .into_iter()
            .flat_map(|key| on_pong(&mut self.wl.xdg_shell, key))
            .collect();
        core_loop::feed(self, events);
    }

    fn client_destroyed(&mut self, client: ShellClient) {
        self.wl.xdg_shell.clients.retain(|_, held| *held != client);
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // A client past its cap is cut (CASC-03); nothing is made for the toplevel it asked for.
        if let Some(client) = surface.wl_surface().client() {
            let held =
                ClientState::of(&client).map_or(0, |data| self.wl.xdg_shell.toplevels_of(data.key));
            if limits::admit(&self.display, &client, Limit::Toplevels, held) == Standing::Over {
                return;
            }
            // The whole session holds a bounded number too: every model step walks every window.
            // When it is full, the client holding the most is the one cut, so a client that fills
            // the session does not get the next innocent one cut at its first window; the asker
            // is cut when it is that client, and then nothing is made for it. An innocent asker
            // is let through: the cut of the largest holder is queued and takes its windows.
            let all = self.core.model.windows().len();
            if Limit::Windows.judge(all) == Standing::Over {
                match self.cut_for_room(&client) {
                    Cut::Other(heaviest) => limits::cut(&heaviest, Limit::Windows),
                    Cut::Asker => {
                        limits::cut(&client, Limit::Windows);
                        return;
                    }
                }
            }
        }
        // No configure here: the initial configure goes out with the first commit.
        let window = self.mint_window();
        let events = on_new_toplevel(&mut self.wl.xdg_shell, &surface, window);
        core_loop::feed(self, events);
    }

    fn new_popup(&mut self, surface: PopupSurface, positioner: PositionerState) {
        popups::new_popup(self, surface, positioner);
    }

    fn move_request(&mut self, surface: ToplevelSurface, _seat: WlSeat, serial: Serial) {
        let serial = serial_of(serial);
        if !self.serial_proves(surface.wl_surface(), serial, GrabUse::Drag) {
            tracing::debug!(?serial, "a move that answers no press is ignored");
            return;
        }
        let events = on_move(&mut self.wl.xdg_shell, &surface, serial);
        core_loop::feed(self, events);
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        _seat: WlSeat,
        serial: Serial,
        edges: ResizeEdge,
    ) {
        let serial = serial_of(serial);
        let Some(edges) = edges_of(edges) else {
            return;
        };
        if !self.serial_proves(surface.wl_surface(), serial, GrabUse::Drag) {
            tracing::debug!(?serial, "a resize that answers no press is ignored");
            return;
        }
        let events = on_resize(&mut self.wl.xdg_shell, &surface, serial, edges);
        core_loop::feed(self, events);
    }

    fn grab(&mut self, surface: PopupSurface, seat: WlSeat, serial: Serial) {
        popups::grab(self, surface, seat, serial);
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        let events = on_maximise(
            &mut self.wl.xdg_shell,
            &surface,
            Toggle::On,
            self.core.model.opening_room(),
        );
        core_loop::feed(self, events);
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        let events = on_maximise(&mut self.wl.xdg_shell, &surface, Toggle::Off, None);
        core_loop::feed(self, events);
    }

    fn fullscreen_request(&mut self, surface: ToplevelSurface, output: Option<WlOutput>) {
        let output = output
            .as_ref()
            .and_then(|output| self.output_key_by_name(output));
        let events = on_full_screen(
            &mut self.wl.xdg_shell,
            &surface,
            Toggle::On,
            output,
            self.core.model.opening_room(),
        );
        core_loop::feed(self, events);
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        let events = on_full_screen(&mut self.wl.xdg_shell, &surface, Toggle::Off, None, None);
        core_loop::feed(self, events);
    }

    fn minimize_request(&mut self, surface: ToplevelSurface) {
        let events = on_minimise(&mut self.wl.xdg_shell, &surface);
        core_loop::feed(self, events);
    }

    fn show_window_menu(
        &mut self,
        surface: ToplevelSurface,
        _seat: WlSeat,
        serial: Serial,
        location: Point<i32, Logical>,
    ) {
        if !self.serial_proves(surface.wl_surface(), serial_of(serial), GrabUse::Drag) {
            return;
        }
        let events = on_window_menu(&mut self.wl.xdg_shell, &surface, point_of(location));
        core_loop::feed(self, events);
    }

    fn ack_configure(&mut self, surface: WlSurface, configure: Configure) {
        if let Configure::Popup(configure) = configure {
            popups::acked(self, &surface, configure);
        }
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        popups::reposition(self, surface, positioner, token);
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        xdg_foreign::on_surface_destroyed(&self.wl.xdg_foreign, surface.wl_surface());
        if let Some(window) = self.wl.xdg_shell.window_of_surface(surface.wl_surface()) {
            // Its buffer may still be attached: the window's last image is kept before it goes.
            core_loop::keep_body_before_destroy(self, window);
            xdg_toplevel_tag::on_forgotten(&mut self.wl.xdg_toplevel_tag, window);
            self.wl.quire_window.windows.remove(&window);
            self.wl.quire_window.early.remove(&window);
            self.wl.xdg_decoration.forget(window);
        }
        let events = on_toplevel_destroyed(&mut self.wl.xdg_shell, &surface);
        core_loop::feed(self, events);
    }

    fn popup_destroyed(&mut self, surface: PopupSurface) {
        let held = holds_the_keyboard(self, &surface);
        let events = xdg_shell::on_popup_destroyed(&mut self.wl.xdg_shell, &surface);
        // smithay counts a grab as active until its popups are cleaned up: without this a
        // menu its client closed (Escape) kept the pointer grab open, and the next click was
        // taken as one outside the menu and swallowed.
        self.wl.xdg_shell.popups.cleanup();
        core_loop::feed(self, events);
        if held {
            // The menu's own client closed it: its grab is over, and smithay would give the
            // keyboard to the menu's root (a bar that never asked for it) at the next key.
            crate::exec::end_popup_keyboard(self);
        }
        // The pointer leaves the popup while its surface still lives (the client destroys the
        // xdg_popup first): a client that is told of an enter with no leave before it, as the
        // surface was destroyed under the pointer, is in a state the protocol does not allow.
        core_loop::refocus_pointer(self);
    }

    fn app_id_changed(&mut self, surface: ToplevelSurface) {
        let events = on_app_id(&mut self.wl.xdg_shell, &surface);
        core_loop::feed(self, events);
    }

    fn title_changed(&mut self, surface: ToplevelSurface) {
        let events = on_title(&mut self.wl.xdg_shell, &surface);
        core_loop::feed(self, events);
    }

    fn parent_changed(&mut self, surface: ToplevelSurface) {
        let events = on_parent(&mut self.wl.xdg_shell, &surface);
        core_loop::feed(self, events);
    }
}

/// Whether `popup` is the first popup of a grab chain and holds the keyboard grab: when it goes,
/// the grab is over. A nested popup that goes hands the grab back to its parent popup.
fn holds_the_keyboard(state: &State, popup: &PopupSurface) -> bool {
    let keyboard = &state.wl.seat.keyboard;
    let kind = PopupKind::Xdg(popup.clone());
    keyboard.is_grabbed()
        && keyboard.current_focus().as_ref() == Some(popup.wl_surface())
        && popup.get_parent_surface() == find_popup_root_surface(&kind).ok()
}
