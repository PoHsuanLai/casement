//! `xdg_wm_base`: toplevels and popups. A toplevel's id is minted when it is created; it
//! enters the model when it maps (its first commit with a buffer after the initial
//! configure). P0 places floating windows only (centre, then cascade, in the model and
//! layout); this module reports what the client did and carries the model's configures out.
//!
//! The module is a directory of small files behind one name:
//! - [`commit`]: what a commit means (initial configure, map, resize, unmap) and the pure
//!   step table behind it;
//! - [`declared`]: what a client declared (title, app id, role, limits) as our types;
//! - [`meta`]: the metadata requests (title, app id, parent, limits);
//! - [`requests`]: move, resize, zoom, minimise, full screen, window menu and the press
//!   proof a grab needs;
//! - [`popup`]: popups, grabs and repositioning;
//! - [`apply`]: configure, close, ping and popup-done onto the wire.

mod apply;
mod commit;
mod declared;
mod meta;
mod popup;
mod requests;

pub use apply::{apply_close, apply_configure, apply_ping, apply_popup_done};
pub use commit::{
    Buffer, CommitStep, Configured, Phase, ToplevelRecord, commit_step, on_new_toplevel,
    on_toplevel_commit, on_toplevel_destroyed,
};
pub use declared::{claimed_of, limits_of, modality_of, role_of};
pub use meta::{base_role, on_app_id, on_limits, on_parent, on_title};
pub use popup::{
    PopupBounds, on_new_popup, on_popup_acked, on_popup_commit, on_popup_destroyed, on_popup_grab,
    on_reposition, on_room_changed, popups_of,
};
pub use requests::{
    GrabUse, edges_of, grab_proven, on_full_screen, on_maximise, on_minimise, on_move, on_pong,
    on_resize, on_window_menu,
};

use crate::states::GlobalFilter;
use casement_core::focus::TokenCheck;
use casement_core::{ClientKey, PopupKey, WindowId};
use smithay::desktop::PopupManager;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::XdgToplevel;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_wm_base::XdgWmBase;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::shell::xdg::{PopupSurface, ShellClient, ToplevelSurface, XdgShellState};
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
    /// Shell clients by key, for pings (a client is recorded when it creates a surface:
    /// smithay's `ShellClient` does not say whose it is).
    pub clients: BTreeMap<ClientKey, ShellClient>,
    /// What the edge remembers of each toplevel between its commits.
    pub records: BTreeMap<WindowId, ToplevelRecord>,
    next_popup: u64,
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
            records: BTreeMap::new(),
            next_popup: 1,
        }
    }

    /// The next popup key (a key is never reused in a session).
    pub fn mint_popup(&mut self) -> PopupKey {
        let key = PopupKey(self.next_popup);
        self.next_popup = self.next_popup.saturating_add(1);
        key
    }

    /// The window whose root surface is `surface`.
    pub fn window_of_surface(&self, surface: &WlSurface) -> Option<WindowId> {
        self.toplevels
            .iter()
            .find(|(_, toplevel)| toplevel.wl_surface() == surface)
            .map(|(id, _)| *id)
    }

    /// The window an `xdg_toplevel` resource is.
    pub fn window_of_toplevel(&self, toplevel: &XdgToplevel) -> Option<WindowId> {
        self.toplevels
            .iter()
            .find(|(_, held)| held.xdg_toplevel() == toplevel)
            .map(|(id, _)| *id)
    }

    /// Whether `window` has mapped (and not unmapped since).
    pub fn is_mapped(&self, window: WindowId) -> bool {
        self.records
            .get(&window)
            .is_some_and(|record| matches!(record.phase, Phase::Mapped(_)))
    }

    /// Keeps the activation verdict a window asked with before it mapped; the model learns
    /// it in `ToplevelMapped` (A7).
    pub fn stage_activation(&mut self, window: WindowId, check: TokenCheck) {
        if let Some(record) = self.records.get_mut(&window) {
            record.staged = check;
        }
    }
}
