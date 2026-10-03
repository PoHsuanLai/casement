//! `ext_session_lock_v1`: the lock screen. While locked, input reaches only lock surfaces
//! (and only physical input, I3); a locker that dies leaves every output covered
//! (`casement_render::LockCoverElement`), never the session revealed.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::geometry::LogicalSize;
use casement_core::policy::GlobalKind;
use casement_core::{Event, OutputKey};
use smithay::reexports::wayland_protocols::ext::session_lock::v1::server::{
    ext_session_lock_manager_v1::ExtSessionLockManagerV1, ext_session_lock_v1::ExtSessionLockV1,
};
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::session_lock::{
    LockSurface, SessionLockHandler, SessionLockManagerGlobalData, SessionLockManagerState,
    SessionLockState, SessionLocker,
};
use std::collections::BTreeMap;

/// `ext_session_lock_manager_v1`, the pending locker and the lock surfaces.
#[derive(Debug)]
pub struct SessionLockPart {
    /// smithay's state.
    pub state: SessionLockManagerState,
    /// The locker waiting for every output to be covered before `locked` is sent.
    pub pending: Option<SessionLocker>,
    /// The lock surface of each output.
    pub surfaces: BTreeMap<OutputKey, LockSurface>,
}

impl SessionLockPart {
    /// Creates the global (version 1), filtered by the I7 table (not for sandboxed clients).
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> SessionLockPart
    where
        D: GlobalDispatch<ExtSessionLockManagerV1, SessionLockManagerGlobalData>
            + Dispatch<ExtSessionLockManagerV1, GlobalData>
            + Dispatch<ExtSessionLockV1, SessionLockState>
            + SessionLockHandler
            + 'static,
    {
        SessionLockPart {
            state: SessionLockManagerState::new::<D, _>(
                display,
                role_filter.for_global(GlobalKind::SessionLock),
            ),
            pending: None,
            surfaces: BTreeMap::new(),
        }
    }
}

/// A client asked to lock the session.
pub fn on_lock(part: &mut SessionLockPart, locker: SessionLocker) -> Vec<Event> {
    let _ = (part, locker);
    todo!("P0-H3: session_lock::on_lock")
}

/// The locker created `surface` for `output`.
pub fn on_lock_surface(
    part: &mut SessionLockPart,
    surface: LockSurface,
    output: Option<OutputKey>,
) -> Vec<Event> {
    let _ = (part, surface, output);
    todo!("P0-H3: session_lock::on_lock_surface")
}

/// The locker unlocked.
pub fn on_unlock(part: &mut SessionLockPart) -> Vec<Event> {
    let _ = part;
    todo!("P0-H3: session_lock::on_unlock")
}

/// The locker's client went away while locked: the session stays locked behind covers.
pub fn on_locker_gone(part: &mut SessionLockPart) -> Vec<Event> {
    let _ = part;
    todo!("P0-H3: session_lock::on_locker_gone")
}

/// Sends `locked` (every output is covered by a lock surface or a cover).
pub fn apply_locked(part: &mut SessionLockPart) -> Result<(), WaylandFault> {
    let _ = part;
    todo!("P0-H3: session_lock::apply_locked")
}

/// Sends `output`'s lock surface its size.
pub fn apply_lock_configure(
    part: &mut SessionLockPart,
    output: &OutputKey,
    size: LogicalSize,
) -> Result<(), WaylandFault> {
    let _ = (part, output, size);
    todo!("P0-H3: session_lock::apply_lock_configure")
}
