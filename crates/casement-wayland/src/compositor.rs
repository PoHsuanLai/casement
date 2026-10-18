//! `wl_compositor`: surfaces and their commits. A commit is classified here by the surface's
//! role and routed by the handler to the module that owns that role (xdg toplevel, popup,
//! layer, lock surface, cursor).

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::reexports::wayland_server::protocol::wl_compositor::WlCompositor;
use smithay::reexports::wayland_server::protocol::wl_subcompositor::WlSubcompositor;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::compositor::{CompositorState, SUBSURFACE_ROLE, get_parent, get_role};
use smithay::wayland::seat::CURSOR_IMAGE_ROLE;
use smithay::wayland::selection::data_device::DND_ICON_ROLE;
use smithay::wayland::shell::wlr_layer::LAYER_SURFACE_ROLE;
use smithay::wayland::shell::xdg::{XDG_POPUP_ROLE, XDG_TOPLEVEL_ROLE};

/// The role string `ext_session_lock_surface_v1` surfaces carry (smithay keeps the constant
/// private).
const LOCK_SURFACE_ROLE: &str = "ext_session_lock_surface_v1";

/// `wl_compositor` and `wl_subcompositor` (smithay creates both globals together).
#[derive(Debug)]
pub struct CompositorPart {
    /// smithay's state.
    pub state: CompositorState,
}

impl CompositorPart {
    /// Creates the `wl_compositor` (version 6) and `wl_subcompositor` globals. Every
    /// connection sees them (I7), so the filter is not consulted.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> CompositorPart
    where
        D: GlobalDispatch<WlCompositor, GlobalData>
            + GlobalDispatch<WlSubcompositor, GlobalData>
            + 'static,
    {
        let _ = role_filter;
        CompositorPart {
            state: CompositorState::new_v6::<D>(display),
        }
    }
}

/// Which module a committed surface belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommitRoute {
    /// An `xdg_toplevel`: `xdg_shell::on_toplevel_commit`.
    Toplevel,
    /// An `xdg_popup`: the popup manager.
    Popup,
    /// A `zwlr_layer_surface_v1`: `layer_shell::on_layer_commit`.
    Layer,
    /// An `ext_session_lock_surface_v1`: `session_lock`.
    LockSurface,
    /// A subsurface: nothing role-specific happens on its own commit (its tree's root commits
    /// carry the role; a synchronised subsurface's state applies with them).
    Subsurface,
    /// A cursor or drag icon.
    Cursor,
    /// No role yet (a commit before the role is assigned), or a role no P0 module owns.
    Unassigned,
}

/// The route a surface with `role` takes (the one table of roles).
pub fn route_of_role(role: Option<&str>) -> CommitRoute {
    match role {
        Some(XDG_TOPLEVEL_ROLE) => CommitRoute::Toplevel,
        Some(XDG_POPUP_ROLE) => CommitRoute::Popup,
        Some(LAYER_SURFACE_ROLE) => CommitRoute::Layer,
        Some(LOCK_SURFACE_ROLE) => CommitRoute::LockSurface,
        Some(SUBSURFACE_ROLE) => CommitRoute::Subsurface,
        Some(CURSOR_IMAGE_ROLE | DND_ICON_ROLE) => CommitRoute::Cursor,
        Some(_) | None => CommitRoute::Unassigned,
    }
}

/// The root of `surface`'s subsurface tree (the surface itself when it has no parent).
pub fn root_of(surface: &WlSurface) -> WlSurface {
    let mut root = surface.clone();
    while let Some(parent) = get_parent(&root) {
        root = parent;
    }
    root
}

/// Classifies a commit on `surface` by its role and says where the role-specific handling
/// goes. smithay's buffer bookkeeping that every role shares (`on_commit_buffer_handler`,
/// which also validates viewports) runs in the handler first: it is generic over the
/// composition root's state. (The plan's sketch returned events; the routing needs the other
/// modules' parts, so the handler routes: FINDINGS.)
pub fn on_commit(part: &mut CompositorPart, surface: &WlSurface) -> CommitRoute {
    let _ = part;
    route_of_role(get_role(surface))
}

/// A surface was destroyed. A plain surface leaves nothing in the model (smithay drops its
/// textures and callbacks with its data); a toplevel's or layer's own destruction arrives
/// through its role module first and carries the event.
pub fn on_destroyed(part: &mut CompositorPart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_role_has_one_route() {
        let rows = [
            (None, CommitRoute::Unassigned),
            (Some("xdg_toplevel"), CommitRoute::Toplevel),
            (Some("xdg_popup"), CommitRoute::Popup),
            (Some("zwlr_layer_surface_v1"), CommitRoute::Layer),
            (
                Some("ext_session_lock_surface_v1"),
                CommitRoute::LockSurface,
            ),
            (Some("subsurface"), CommitRoute::Subsurface),
            (Some("cursor_image"), CommitRoute::Cursor),
            (Some("dnd_icon"), CommitRoute::Cursor),
            (Some("zwp_input_popup_surface_v2"), CommitRoute::Unassigned),
        ];
        for (role, route) in rows {
            assert_eq!(route_of_role(role), route, "{role:?}");
        }
    }
}
