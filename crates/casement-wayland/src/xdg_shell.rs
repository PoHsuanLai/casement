//! `xdg_wm_base`: toplevels and popups. A toplevel's id is minted when it is created; it
//! enters the model when it maps (its first commit with a buffer after the initial
//! configure). P0 places floating windows only (centre, then cascade).

use crate::client::ClientState;
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::geometry::LogicalPoint;
use casement_core::{
    ClientKey, ConfigureSpec, Event, InputSerial, OutputKey, PopupKey, ResizeEdges, WindowId,
};
use casement_protocols::vocab::Toggle;
use smithay::desktop::PopupManager;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_wm_base::XdgWmBase;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ShellClient, ToplevelSurface, XdgShellState,
};
use std::collections::BTreeMap;

/// `xdg_wm_base` and the toplevels and popups it created.
#[derive(Debug)]
pub struct XdgShellPart {
    /// smithay's state.
    pub state: XdgShellState,
    /// smithay's popup tree (grabs, positioning relative to the parent).
    pub popups: PopupManager,
    /// Every live toplevel, mapped or not.
    pub toplevels: BTreeMap<WindowId, ToplevelSurface>,
    /// Every live popup.
    pub popup_surfaces: BTreeMap<PopupKey, PopupSurface>,
    /// Shell clients by key, for pings.
    pub clients: BTreeMap<ClientKey, ShellClient>,
}

impl XdgShellPart {
    /// Creates the `xdg_wm_base` global (version 7, every capability). Every connection sees
    /// it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> XdgShellPart
    where
        D: GlobalDispatch<XdgWmBase, GlobalData> + 'static,
    {
        let _ = role_filter;
        XdgShellPart {
            state: XdgShellState::new::<D>(display),
            popups: PopupManager::default(),
            toplevels: BTreeMap::new(),
            popup_surfaces: BTreeMap::new(),
            clients: BTreeMap::new(),
        }
    }
}

/// A toplevel was created and given the id `window` (no model event until it maps).
pub fn on_new_toplevel(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    window: WindowId,
) -> Vec<Event> {
    let _ = (part, surface, window);
    todo!("P0-H2: xdg_shell::on_new_toplevel")
}

/// A toplevel's surface committed: its initial configure, its mapping
/// (`Event::ToplevelMapped`, with the app `client` proves), or a new size.
pub fn on_toplevel_commit(
    part: &mut XdgShellPart,
    surface: &WlSurface,
    client: &ClientState,
) -> Vec<Event> {
    let _ = (part, surface, client);
    todo!("P0-H2: xdg_shell::on_toplevel_commit")
}

/// An interactive move answering the press `serial`.
pub fn on_move(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    serial: InputSerial,
) -> Vec<Event> {
    let _ = (part, surface, serial);
    todo!("P0-H2: xdg_shell::on_move")
}

/// An interactive resize from `edges` answering the press `serial`.
pub fn on_resize(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    serial: InputSerial,
    edges: ResizeEdges,
) -> Vec<Event> {
    let _ = (part, surface, serial, edges);
    todo!("P0-H2: xdg_shell::on_resize")
}

/// Maximise (zoom) on or off.
pub fn on_maximise(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    toggle: Toggle,
) -> Vec<Event> {
    let _ = (part, surface, toggle);
    todo!("P0-H2: xdg_shell::on_maximise")
}

/// Minimise.
pub fn on_minimise(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H2: xdg_shell::on_minimise")
}

/// Full screen on or off, optionally on `output`.
pub fn on_full_screen(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    toggle: Toggle,
    output: Option<OutputKey>,
) -> Vec<Event> {
    let _ = (part, surface, toggle, output);
    todo!("P0-H2: xdg_shell::on_full_screen")
}

/// Show the window menu at `at` (global logical).
pub fn on_window_menu(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    at: LogicalPoint,
) -> Vec<Event> {
    let _ = (part, surface, at);
    todo!("P0-H2: xdg_shell::on_window_menu")
}

/// The title changed.
pub fn on_title(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H2: xdg_shell::on_title")
}

/// The app id changed.
pub fn on_app_id(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H2: xdg_shell::on_app_id")
}

/// The parent changed (an `xdg_foreign` import counts).
pub fn on_parent(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H2: xdg_shell::on_parent")
}

/// The size limits changed (read on commit).
pub fn on_limits(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H2: xdg_shell::on_limits")
}

/// The toplevel was destroyed.
pub fn on_toplevel_destroyed(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H2: xdg_shell::on_toplevel_destroyed")
}

/// A popup was created and given the key `popup`.
pub fn on_new_popup(
    part: &mut XdgShellPart,
    surface: &PopupSurface,
    positioner: &PositionerState,
    popup: PopupKey,
) -> Vec<Event> {
    let _ = (part, surface, positioner, popup);
    todo!("P0-H2: xdg_shell::on_new_popup")
}

/// A popup asked for an explicit grab answering the press `serial`.
pub fn on_popup_grab(
    part: &mut XdgShellPart,
    surface: &PopupSurface,
    serial: InputSerial,
) -> Vec<Event> {
    let _ = (part, surface, serial);
    todo!("P0-H2: xdg_shell::on_popup_grab")
}

/// A popup asked to be repositioned (`token` goes back with the configure).
pub fn on_reposition(
    part: &mut XdgShellPart,
    surface: &PopupSurface,
    positioner: &PositionerState,
    token: u32,
) -> Vec<Event> {
    let _ = (part, surface, positioner, token);
    todo!("P0-H2: xdg_shell::on_reposition")
}

/// A client answered a ping.
pub fn on_pong(part: &mut XdgShellPart, client: ClientKey) -> Vec<Event> {
    let _ = (part, client);
    todo!("P0-H2: xdg_shell::on_pong")
}

/// Sends `window` a configure.
pub fn apply_configure(
    part: &mut XdgShellPart,
    window: WindowId,
    spec: &ConfigureSpec,
) -> Result<(), WaylandFault> {
    let _ = (part, window, spec);
    todo!("P0-H2: xdg_shell::apply_configure")
}

/// Asks `window` to close.
pub fn apply_close(part: &mut XdgShellPart, window: WindowId) -> Result<(), WaylandFault> {
    let _ = (part, window);
    todo!("P0-H2: xdg_shell::apply_close")
}

/// Pings `client`.
pub fn apply_ping(part: &mut XdgShellPart, client: ClientKey) -> Result<(), WaylandFault> {
    let _ = (part, client);
    todo!("P0-H2: xdg_shell::apply_ping")
}

/// Dismisses `popup` (and the popups above it).
pub fn apply_popup_done(part: &mut XdgShellPart, popup: PopupKey) -> Result<(), WaylandFault> {
    let _ = (part, popup);
    todo!("P0-H2: xdg_shell::apply_popup_done")
}
