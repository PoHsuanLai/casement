//! What one client may hold at once (CASC-03, CASW-06). The loop is single-threaded, so a peer
//! that makes objects without end stalls the whole session; each kind of object has a cap, and a
//! client that asks for one past it is cut (its connection closed, with a log line naming the
//! cap), never allowed to wedge us. The system backend's kill sends no `wl_display` error, and
//! reaching the client's `wl_display` resource to post `no_memory` would need `unsafe`; the log
//! line is where the reason lives.
//!
//! The caps are generous: a person's real clients are an order of magnitude under them. Each
//! call site counts what the client holds from the collection it already keeps, asks
//! [`Limit::judge`], and calls [`cut`] on [`Standing::Over`]. This module is the numbers and the
//! cut; the counting stays with the code that owns the objects.
//!
//! A cut is queued, never done inside the request that went over: killing a client from its own
//! request handler frees the object libwayland is still dispatching (a use after free in the
//! system backend). The composition root calls [`cut_queued`] right after each dispatch.

use std::cell::RefCell;

use smithay::reexports::wayland_server::backend::protocol::ProtocolError;
use smithay::reexports::wayland_server::{Client, DisplayHandle};

thread_local! {
    /// Clients that went over a cap during the dispatch in progress, with the cap: the loop and
    /// its dispatch run on one thread, so the queue is that thread's.
    static QUEUED: RefCell<Vec<(Client, Limit)>> = const { RefCell::new(Vec::new()) };
}

/// `wl_display.error.no_memory`: the protocol's error for a server that cannot take more.
const WL_DISPLAY_NO_MEMORY: u32 = 2;

/// The `wl_display` object's id, which every connection has from its first byte.
const WL_DISPLAY_ID: u32 = 1;

/// Toplevels one client may hold: the X11 satellite carries every X11 window, so this is
/// well above any real set, and low enough that the per-step snapshots stay cheap.
pub const MAX_TOPLEVELS: usize = 1024;

/// Popups (menus, tooltips) one client may hold open.
pub const MAX_POPUPS: usize = 256;

/// Layer surfaces (bars, wallpapers, overlays) one client may hold.
pub const MAX_LAYERS: usize = 64;

/// Screencopy frames one client may have waiting: each is a full read-back of an output on
/// the loop, on every frame presented.
pub const MAX_PENDING_COPIES: usize = 16;

/// Window-list managers (`zwlr_foreign_toplevel_manager_v1`) one client may bind: each bind
/// lists every window again and is told every title change.
pub const MAX_WINDOW_LIST_MANAGERS: usize = 4;

/// Icon buffers (`xdg_toplevel_icon_v1.add_buffer`) one client may hold: every buffer
/// destroy by any client scans the table they sit in.
pub const MAX_ICON_BUFFERS: usize = 256;

/// Foreign-toplevel exports (`zxdg_exporter_v2`) one client may hold.
pub const MAX_EXPORTS: usize = 256;

/// Keymaps one virtual keyboard may send: each is up to a mebibyte compiled by libxkbcommon on
/// the loop, and a keyboard sets its keymap once or when the layout changes.
pub const MAX_KEYMAPS: usize = 16;

/// The objects whose count per client is capped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Limit {
    /// `xdg_toplevel`.
    Toplevels,
    /// `xdg_popup`.
    Popups,
    /// `zwlr_layer_surface_v1`.
    Layers,
    /// Waiting `zwlr_screencopy_frame_v1`.
    PendingCopies,
    /// `zwlr_foreign_toplevel_manager_v1`.
    WindowListManagers,
    /// Buffers held by `xdg_toplevel_icon_v1` objects.
    IconBuffers,
    /// `zxdg_exported_v2`.
    Exports,
    /// `zwp_virtual_keyboard_v1.keymap` requests of one keyboard.
    Keymaps,
}

/// Whether one more object fits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// It fits.
    Within,
    /// The client already holds the most it may.
    Over,
}

impl Limit {
    /// The most a client may hold of this object.
    pub const fn cap(self) -> usize {
        match self {
            Limit::Toplevels => MAX_TOPLEVELS,
            Limit::Popups => MAX_POPUPS,
            Limit::Layers => MAX_LAYERS,
            Limit::PendingCopies => MAX_PENDING_COPIES,
            Limit::WindowListManagers => MAX_WINDOW_LIST_MANAGERS,
            Limit::IconBuffers => MAX_ICON_BUFFERS,
            Limit::Exports => MAX_EXPORTS,
            Limit::Keymaps => MAX_KEYMAPS,
        }
    }

    /// What is capped, for the error text the client reads.
    pub const fn what(self) -> &'static str {
        match self {
            Limit::Toplevels => "toplevel windows",
            Limit::Popups => "popups",
            Limit::Layers => "layer surfaces",
            Limit::PendingCopies => "screen copies waiting",
            Limit::WindowListManagers => "window list managers",
            Limit::IconBuffers => "icon buffers",
            Limit::Exports => "exported windows",
            Limit::Keymaps => "keymaps",
        }
    }

    /// Whether a client that holds `held` of these may take one more.
    pub const fn judge(self, held: usize) -> Standing {
        match held < self.cap() {
            true => Standing::Within,
            false => Standing::Over,
        }
    }
}

/// Whether `client`, which holds `held` of `limit`, may take one more; a client that may not is
/// queued to be cut ([`cut`]) before this returns [`Standing::Over`], and the caller makes nothing.
pub fn admit(_display: &DisplayHandle, client: &Client, limit: Limit, held: usize) -> Standing {
    let standing = limit.judge(held);
    if standing == Standing::Over {
        cut(client, limit);
    }
    standing
}

/// Queues `client` to be cut: it asked for more `limit` than it may hold. The cut happens at [`cut_queued`], after the dispatch that asked; a client queued twice
/// is cut once.
pub fn cut(client: &Client, limit: Limit) {
    QUEUED.with(|queued| {
        let mut queued = queued.borrow_mut();
        if queued.iter().all(|(held, _)| held.id() != client.id()) {
            queued.push((client.clone(), limit));
        }
    });
}

/// Cuts every client queued by [`cut`] since the last call. Called by the composition root after
/// each dispatch of client requests, never from inside one.
pub fn cut_queued(display: &DisplayHandle) {
    let queued = QUEUED.with(|queued| std::mem::take(&mut *queued.borrow_mut()));
    for (client, limit) in queued {
        tracing::warn!(
            client = ?client.id(),
            "a client asked for too many {} (at most {}); its connection is closed",
            limit.what(),
            limit.cap()
        );
        client.kill(
            display,
            ProtocolError {
                code: WL_DISPLAY_NO_MEMORY,
                object_id: WL_DISPLAY_ID,
                object_interface: "wl_display".to_owned(),
                message: format!("too many {} (at most {})", limit.what(), limit.cap()),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The last object that fits is the cap-th; the next is over.
    #[test]
    fn a_client_may_hold_up_to_the_cap_and_no_more() {
        let all = [
            Limit::Toplevels,
            Limit::Popups,
            Limit::Layers,
            Limit::PendingCopies,
            Limit::WindowListManagers,
            Limit::IconBuffers,
            Limit::Exports,
            Limit::Keymaps,
        ];
        for limit in all {
            assert_eq!(limit.judge(0), Standing::Within, "{limit:?}");
            assert_eq!(limit.judge(limit.cap() - 1), Standing::Within, "{limit:?}");
            assert_eq!(limit.judge(limit.cap()), Standing::Over, "{limit:?}");
            assert_eq!(limit.judge(usize::MAX), Standing::Over, "{limit:?}");
        }
    }
}
