//! `xdg_toplevel_icon_manager_v1`: a toplevel's own icon (the dock and switcher prefer the
//! app's installed icon; this is the fallback).

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::XdgToplevel;
use smithay::reexports::wayland_protocols::xdg::toplevel_icon::v1::server::xdg_toplevel_icon_manager_v1::XdgToplevelIconManagerV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::xdg_toplevel_icon::{XdgToplevelIconHandler, XdgToplevelIconManager, XdgToplevelIconManagerUserData};

/// `xdg_toplevel_icon_manager_v1`.
#[derive(Debug)]
pub struct XdgToplevelIconPart {
    /// smithay's state.
    pub state: XdgToplevelIconManager,
}

impl XdgToplevelIconPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> XdgToplevelIconPart
    where
        D: XdgToplevelIconHandler
            + GlobalDispatch<XdgToplevelIconManagerV1, XdgToplevelIconManagerUserData>,
    {
        let _ = role_filter;
        XdgToplevelIconPart {
            state: XdgToplevelIconManager::new::<D>(display),
        }
    }
}

/// `toplevel` set (or cleared) its icon; `surface` is its root surface.
pub fn on_icon(
    part: &mut XdgToplevelIconPart,
    toplevel: &XdgToplevel,
    surface: &WlSurface,
) -> Vec<Event> {
    let _ = (part, toplevel, surface);
    todo!("P0-H2: xdg_toplevel_icon::on_icon")
}
