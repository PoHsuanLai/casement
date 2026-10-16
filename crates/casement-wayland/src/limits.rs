//! What one client may hold at once (CASC-03, CASW-06). The loop is single-threaded, so a peer
//! that makes objects without end stalls the whole session; each kind of object has a cap, and a
//! client that asks for one past it is cut with a protocol error (`wl_display.no_memory`, the
//! error the protocol has for a server that cannot take more), never allowed to wedge us.
//!
//! The caps are generous: a person's real clients are an order of magnitude under them. Each
//! call site counts what the client holds from the collection it already keeps, asks
//! [`Limit::judge`], and calls [`cut`] on [`Standing::Over`]. This module is the numbers and the
//! cut; the counting stays with the code that owns the objects.

use smithay::reexports::wayland_server::backend::protocol::ProtocolError;
use smithay::reexports::wayland_server::{Client, DisplayHandle};

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
/// cut ([`cut`]) before this returns [`Standing::Over`], and the caller makes nothing.
pub fn admit(display: &DisplayHandle, client: &Client, limit: Limit, held: usize) -> Standing {
    let standing = limit.judge(held);
    if standing == Standing::Over {
        cut(display, client, limit);
    }
    standing
}

/// Cuts `client` with `wl_display.no_memory`: it asked for more `limit` than it may hold.
pub fn cut(display: &DisplayHandle, client: &Client, limit: Limit) {
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
        ];
        for limit in all {
            assert_eq!(limit.judge(0), Standing::Within, "{limit:?}");
            assert_eq!(limit.judge(limit.cap() - 1), Standing::Within, "{limit:?}");
            assert_eq!(limit.judge(limit.cap()), Standing::Over, "{limit:?}");
            assert_eq!(limit.judge(usize::MAX), Standing::Over, "{limit:?}");
        }
    }
}
