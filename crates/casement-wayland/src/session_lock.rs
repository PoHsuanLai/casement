//! `ext_session_lock_v1`: the lock screen. While locked, input reaches only lock surfaces
//! (and only physical input, I3); a locker that dies leaves every output covered
//! (`casement_render::LockCoverElement`), never the session revealed.
//!
//! The flow: `lock` stores the requesting locker and tells the model (`LockEvent::Requested`);
//! the model draws covers and, once the locker's first surface has content, answers
//! `LockReply::SendLocked`, which [`apply_locked`] carries out. smithay has no callback for a
//! locker that disconnects, so the part remembers the lock object and [`poll_locker`] notices
//! it is dead (the handler calls it when a lock surface is destroyed and the loop once a turn).
//!
//! Locking is a power (`Power::Lock`): a `lock` asked for while the person has not answered is
//! kept in [`SessionLockPart::hold`], and dropped (the client is told `finished`) when they
//! refuse. A lock that already holds the session is never ended because the power is taken
//! back: the covers stay up, and only the locker can unlock.

use crate::client::ClientState;
use crate::consent::{Held, Mortal, Pushed};
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::ClientKey;
use casement_core::geometry::LogicalSize;
use casement_core::lock::LockEvent;
use casement_core::policy::GlobalKind;
use casement_core::{Event, OutputKey};
use smithay::reexports::wayland_protocols::ext::session_lock::v1::server::{
    ext_session_lock_manager_v1::ExtSessionLockManagerV1, ext_session_lock_v1::ExtSessionLockV1,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch, Resource};
use smithay::wayland::GlobalData;
use smithay::wayland::compositor::{self, BufferAssignment, SurfaceAttributes};
use smithay::wayland::session_lock::{
    LockSurface, SessionLockHandler, SessionLockManagerGlobalData, SessionLockManagerState,
    SessionLockState, SessionLocker,
};
use std::collections::{BTreeMap, BTreeSet};

/// `ext_session_lock_manager_v1`, the pending locker and the lock surfaces.
#[derive(Debug)]
pub struct SessionLockPart {
    /// smithay's state.
    pub state: SessionLockManagerState,
    /// The locker waiting for every output to be covered before `locked` is sent.
    pub pending: Option<SessionLocker>,
    /// The lock surface of each output.
    pub surfaces: BTreeMap<OutputKey, LockSurface>,
    /// The lock object that holds the session, once `locked` was sent.
    pub held: Option<ExtSessionLockV1>,
    /// The outputs whose lock surface has shown content (reported to the model once each).
    pub shown: BTreeSet<OutputKey>,
    /// The lockers that asked while the person had not answered (`Power::Lock`).
    gated: Held<SessionLocker>,
}

impl Mortal for SessionLocker {
    fn alive(&self) -> bool {
        self.ext_session_lock().is_alive()
    }
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
            held: None,
            shown: BTreeSet::new(),
            gated: Held::default(),
        }
    }

    /// Keeps `locker`, whose client is `client`, until the answer about its power to lock
    /// comes out; one past the cap of waiting requests is dropped, which tells it `finished`.
    pub fn hold(&mut self, client: ClientKey, locker: SessionLocker) {
        if let Pushed::Full(locker) = self.gated.push(client, locker) {
            drop(locker);
        }
    }

    /// The lockers `client` has waiting, oldest first.
    pub fn take_gated(&mut self, client: ClientKey) -> Vec<SessionLocker> {
        self.gated.take(client)
    }

    /// The output whose lock surface is `surface`.
    pub fn output_of(&self, surface: &WlSurface) -> Option<OutputKey> {
        self.surfaces
            .iter()
            .find(|(_, lock)| lock.wl_surface() == surface)
            .map(|(output, _)| output.clone())
    }

    /// Whether a locker holds or is taking the session (a second `lock` is refused).
    fn taken(&self) -> bool {
        self.pending.is_some() || self.held.as_ref().is_some_and(Resource::is_alive)
    }
}

/// A client asked to lock the session. A request while another locker holds or is taking the
/// session is refused (the newcomer is dropped, which sends it `finished`); a locker that
/// died since is reported first, so the newcomer takes the lock over.
pub fn on_lock(part: &mut SessionLockPart, locker: SessionLocker) -> Vec<Event> {
    let mut events = poll_locker(part);
    let client = locker
        .ext_session_lock()
        .client()
        .and_then(|client| ClientState::of(&client).map(|state| state.key));
    match (client, part.taken()) {
        (Some(client), false) => {
            part.pending = Some(locker);
            events.push(Event::Lock(LockEvent::Requested { client }));
        }
        (Some(_), true) | (None, _) => drop(locker),
    }
    events
}

/// The locker created `surface` for `output`; it counts once it shows content
/// ([`on_lock_surface_commit`]).
pub fn on_lock_surface(
    part: &mut SessionLockPart,
    surface: LockSurface,
    output: Option<OutputKey>,
) -> Vec<Event> {
    if let Some(output) = output {
        part.shown.remove(&output);
        part.surfaces.insert(output, surface);
    }
    Vec::new()
}

/// What a commit did to a lock surface's buffer.
enum Attach {
    /// A new buffer was attached.
    Content,
    /// The buffer was removed (`attach(null)`): the surface shows nothing.
    Removed,
    /// Neither: the commit left the buffer as it was.
    Unchanged,
}

/// A lock surface committed: the first commit with a buffer on an output tells the model the
/// locker covers it (`LockEvent::SurfaceCommitted`); one that removes its buffer no longer covers it
/// (`LockEvent::SurfaceGone`).
pub fn on_lock_surface_commit(part: &mut SessionLockPart, surface: &WlSurface) -> Vec<Event> {
    let Some(output) = part.output_of(surface) else {
        return Vec::new();
    };
    let attach = compositor::with_states(surface, |states| {
        let mut attributes = states.cached_state.get::<SurfaceAttributes>();
        match attributes.current().buffer {
            Some(BufferAssignment::NewBuffer(_)) => Attach::Content,
            Some(BufferAssignment::Removed) => Attach::Removed,
            None => Attach::Unchanged,
        }
    });
    match attach {
        Attach::Content if part.shown.insert(output.clone()) => {
            vec![Event::Lock(LockEvent::SurfaceCommitted { output })]
        }
        Attach::Content | Attach::Unchanged => Vec::new(),
        Attach::Removed => surface_gone(part, &output),
    }
}

/// A lock surface was destroyed: its output is no longer covered by the locker (the cover
/// goes back up there), and a locker that went with it is reported.
pub fn on_lock_surface_destroyed(part: &mut SessionLockPart, surface: &WlSurface) -> Vec<Event> {
    let mut events = match part.output_of(surface) {
        Some(output) => {
            part.surfaces.remove(&output);
            surface_gone(part, &output)
        }
        None => Vec::new(),
    };
    events.extend(poll_locker(part));
    events
}

/// The model hears that `output`'s lock surface stopped covering it, if it had been told that
/// it did; `shown` forgets the output either way.
fn surface_gone(part: &mut SessionLockPart, output: &OutputKey) -> Vec<Event> {
    match part.shown.remove(output) {
        true => vec![Event::Lock(LockEvent::SurfaceGone {
            output: output.clone(),
        })],
        false => Vec::new(),
    }
}

/// The locker unlocked.
pub fn on_unlock(part: &mut SessionLockPart) -> Vec<Event> {
    part.held = None;
    part.pending = None;
    part.surfaces.clear();
    part.shown.clear();
    vec![Event::Lock(LockEvent::Unlocked)]
}

/// The locker's client went away while locked: the session stays locked behind covers.
pub fn on_locker_gone(part: &mut SessionLockPart) -> Vec<Event> {
    let had_locker = part.held.take().is_some() | part.pending.take().is_some();
    part.surfaces.clear();
    part.shown.clear();
    if had_locker {
        vec![Event::Lock(LockEvent::LockerGone)]
    } else {
        Vec::new()
    }
}

/// Notices a locker whose connection is gone (smithay has no callback for it): its lock
/// object, or the one still waiting to lock, is dead.
pub fn poll_locker(part: &mut SessionLockPart) -> Vec<Event> {
    let held_dead = part.held.as_ref().is_some_and(|lock| !lock.is_alive());
    let pending_dead = part
        .pending
        .as_ref()
        .is_some_and(|locker| !locker.ext_session_lock().is_alive());
    if held_dead || pending_dead {
        on_locker_gone(part)
    } else {
        Vec::new()
    }
}

/// Sends `locked` (every output is covered by a lock surface or a cover).
pub fn apply_locked(part: &mut SessionLockPart) -> Result<(), WaylandFault> {
    let locker = part.pending.take().ok_or(WaylandFault::SurfaceGone)?;
    part.held = Some(locker.ext_session_lock().clone());
    locker.lock();
    Ok(())
}

/// The lock is over for the locker still waiting: dropping it sends `finished`.
pub fn apply_finished(part: &mut SessionLockPart) {
    part.pending = None;
}

/// Sends `output`'s lock surface its size.
pub fn apply_lock_configure(
    part: &mut SessionLockPart,
    output: &OutputKey,
    size: LogicalSize,
) -> Result<(), WaylandFault> {
    let surface = part.surfaces.get(output).ok_or(WaylandFault::SurfaceGone)?;
    if !surface.alive() {
        return Err(WaylandFault::SurfaceGone);
    }
    surface.with_pending_state(|state| state.size = Some((size.w, size.h).into()));
    surface.send_configure();
    Ok(())
}
