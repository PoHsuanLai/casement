//! `wl_compositor`: surfaces and their commits. A commit is classified here by the surface's
//! role and routed by the handler to the module that owns that role (xdg toplevel, popup,
//! layer, lock surface, cursor, subsurface).

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::reexports::wayland_server::protocol::wl_compositor::WlCompositor;
use smithay::reexports::wayland_server::protocol::wl_subcompositor::WlSubcompositor;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::compositor::CompositorState;

/// `wl_compositor` and `wl_subcompositor` (smithay creates both globals together).
#[derive(Debug)]
pub struct CompositorPart {
    /// smithay's state.
    pub state: CompositorState,
}

impl CompositorPart {
    /// Creates the `wl_compositor` (version 5) and `wl_subcompositor` globals. Every
    /// connection sees them (I7), so the filter is not consulted.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> CompositorPart
    where
        D: GlobalDispatch<WlCompositor, GlobalData>
            + GlobalDispatch<WlSubcompositor, GlobalData>
            + 'static,
    {
        let _ = role_filter;
        CompositorPart {
            state: CompositorState::new::<D>(display),
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
    /// A subsurface: its root's route decides.
    Subsurface,
    /// A cursor or drag icon.
    Cursor,
    /// No role yet (a commit before the role is assigned).
    Unassigned,
}

/// Handles a commit on `surface`: runs smithay's buffer bookkeeping that every role shares
/// and says where the role-specific handling goes. (The plan's sketch returned events; the
/// routing needs the other modules' parts, so the handler routes: FINDINGS.)
pub fn on_commit(part: &mut CompositorPart, surface: &WlSurface) -> CommitRoute {
    let _ = (part, surface);
    todo!("P0-H1: compositor::on_commit")
}

/// A surface was destroyed: what the model must forget (a toplevel's or layer's own
/// destruction arrives through its role module first).
pub fn on_destroyed(part: &mut CompositorPart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H1: compositor::on_destroyed")
}
