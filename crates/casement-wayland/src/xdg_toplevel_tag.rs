//! `xdg_toplevel_tag_manager_v1`: a stable tag per toplevel, the key for window placement
//! memory (`$XDG_STATE_HOME/casement/windows.toml`, C52).

use crate::states::GlobalFilter;
use casement_core::{Event, WindowId};
use smithay::reexports::wayland_protocols::xdg::toplevel_tag::v1::server::xdg_toplevel_tag_manager_v1::XdgToplevelTagManagerV1;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::xdg_toplevel_tag::{XdgToplevelTagHandler, XdgToplevelTagManager};
use std::collections::BTreeMap;

/// `xdg_toplevel_tag_manager_v1`.
#[derive(Debug)]
pub struct XdgToplevelTagPart {
    /// smithay's state.
    pub state: XdgToplevelTagManager,
    /// Each window's tag, for the placement memory (C52, P3).
    pub tags: BTreeMap<WindowId, ToplevelTag>,
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
            tags: BTreeMap::new(),
        }
    }
}

/// A toplevel's tag, as the client gave it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToplevelTag(pub String);

/// `window` set its tag: kept for the placement memory. The model has no tag, so nothing is
/// reported.
pub fn on_tag(part: &mut XdgToplevelTagPart, window: WindowId, tag: ToplevelTag) -> Vec<Event> {
    part.tags.insert(window, tag);
    Vec::new()
}

/// `window` is gone: its tag is forgotten.
pub fn on_forgotten(part: &mut XdgToplevelTagPart, window: WindowId) {
    part.tags.remove(&window);
}
