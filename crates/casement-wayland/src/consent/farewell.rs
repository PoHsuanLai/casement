//! Destructor events a bind cannot send. `finished` on a manager destroys the object, and
//! libwayland's bind path writes the new object's data after the bind handler returns, so a
//! manager refused at bind is finished right after the dispatch instead (as
//! [`limits::cut_queued`](crate::limits::cut_queued) cuts clients).

use smithay::reexports::wayland_protocols::ext::workspace::v1::server::ext_workspace_manager_v1::ExtWorkspaceManagerV1;
use smithay::reexports::wayland_protocols_wlr::foreign_toplevel::v1::server::zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1;
use smithay::reexports::wayland_server::Resource;
use std::cell::RefCell;

/// A manager to finish once the dispatch in progress is over.
#[derive(Debug)]
pub enum Farewell {
    /// A `zwlr_foreign_toplevel_manager_v1`.
    WindowList(ZwlrForeignToplevelManagerV1),
    /// An `ext_workspace_manager_v1`.
    Workspaces(ExtWorkspaceManagerV1),
}

thread_local! {
    /// Managers refused during the dispatch in progress: the loop and its dispatch run on one
    /// thread, so the queue is that thread's.
    static QUEUED: RefCell<Vec<Farewell>> = const { RefCell::new(Vec::new()) };
}

/// Finishes `farewell` after the dispatch in progress.
pub fn later(farewell: Farewell) {
    QUEUED.with(|queued| queued.borrow_mut().push(farewell));
}

/// Sends every queued `finished`, outside the dispatch. The composition root calls it after
/// each dispatch of the clients.
pub fn send_queued() {
    let queued = QUEUED.with(|queued| std::mem::take(&mut *queued.borrow_mut()));
    for farewell in queued {
        match farewell {
            Farewell::WindowList(manager) if manager.is_alive() => manager.finished(),
            Farewell::Workspaces(manager) if manager.is_alive() => manager.finished(),
            _ => {}
        }
    }
}
