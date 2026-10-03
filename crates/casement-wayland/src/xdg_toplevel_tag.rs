//! `xdg_toplevel_tag_manager_v1`: a stable tag per toplevel, the key for window placement
//! memory (`$XDG_STATE_HOME/casement/windows.toml`, C52).

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::XdgToplevel;
use smithay::reexports::wayland_protocols::xdg::toplevel_tag::v1::server::xdg_toplevel_tag_manager_v1::XdgToplevelTagManagerV1;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::xdg_toplevel_tag::{XdgToplevelTagHandler, XdgToplevelTagManager};

/// `xdg_toplevel_tag_manager_v1`.
#[derive(Debug)]
pub struct XdgToplevelTagPart {
    /// smithay's state.
    pub state: XdgToplevelTagManager,
}

impl XdgToplevelTagPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> XdgToplevelTagPart
    where
        D: XdgToplevelTagHandler + GlobalDispatch<XdgToplevelTagManagerV1, GlobalData>,
    {
        let _ = role_filter;
        XdgToplevelTagPart {
            state: XdgToplevelTagManager::new::<D>(display),
        }
    }
}

/// A toplevel's tag, as the client gave it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToplevelTag(pub String);

/// `toplevel` set its tag.
pub fn on_tag(
    part: &mut XdgToplevelTagPart,
    toplevel: &XdgToplevel,
    tag: ToplevelTag,
) -> Vec<Event> {
    let _ = (part, toplevel, tag);
    todo!("P0-H2: xdg_toplevel_tag::on_tag")
}
