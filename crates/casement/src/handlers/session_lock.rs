//! `ext_session_lock_manager_v1` (P0-H3).

use super::layer_shell::output_key_of;
use crate::core_loop;
use crate::state::State;
use casement_core::geometry::LogicalSize;
use casement_core::privilege::PowerOutcome;
use casement_core::{ClientKey, OutputKey};
use casement_protocols::vocab::Power;
use casement_wayland::consent::{self, Admission};
use casement_wayland::session_lock;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::wayland::compositor;
use smithay::wayland::session_lock::{
    LockSurface, SessionLockHandler, SessionLockManagerState, SessionLocker,
};

/// The size of `output`, if the model has it plugged.
fn size_of_output(state: &State, output: &OutputKey) -> Option<LogicalSize> {
    state
        .core
        .model
        .outputs()
        .0
        .get(output)
        .map(|plugged| plugged.config.logical_rect().size)
}

/// `locker` when its client may lock the session now. Otherwise it is kept until the person
/// answers (`None`), or dropped, which tells the client `finished`.
fn pass(state: &mut State, locker: SessionLocker) -> Option<SessionLocker> {
    let client = locker.ext_session_lock().client()?;
    match consent::admit(state, &client, Power::Lock) {
        Admission::Go => Some(locker),
        Admission::Hold => {
            if let Some(key) = consent::key_of(&client) {
                state.wl.session_lock.hold(key, locker);
            }
            None
        }
        Admission::Refuse => None,
    }
}

/// The answer about `client`'s power to lock came out: the lockers that waited ask again and
/// are let through, or are dropped (`finished`). A lock already held stays.
pub(super) fn resolved(state: &mut State, client: ClientKey, outcome: PowerOutcome) {
    let lockers = state.wl.session_lock.take_gated(client);
    match outcome {
        PowerOutcome::Allowed => {
            for locker in lockers {
                SessionLockHandler::lock(state, locker);
            }
        }
        PowerOutcome::Denied | PowerOutcome::Revoked => drop(lockers),
    }
}

impl SessionLockHandler for State {
    fn lock_state(&mut self) -> &mut SessionLockManagerState {
        &mut self.wl.session_lock.state
    }

    fn lock(&mut self, confirmation: SessionLocker) {
        let Some(confirmation) = pass(self, confirmation) else {
            return;
        };
        let events = session_lock::on_lock(&mut self.wl.session_lock, confirmation);
        core_loop::feed(self, events);
    }

    fn unlock(&mut self) {
        let events = session_lock::on_unlock(&mut self.wl.session_lock);
        core_loop::feed(self, events);
    }

    fn new_surface(&mut self, surface: LockSurface, output: WlOutput) {
        let key = output_key_of(self, &output);
        // smithay sends the initial configure when this returns: ask for the whole output.
        if let Some(size) = key.as_ref().and_then(|key| size_of_output(self, key)) {
            surface.with_pending_state(|pending| pending.size = Some((size.w, size.h).into()));
        }
        let wl_surface = surface.wl_surface().clone();
        compositor::add_post_commit_hook::<State, _>(&wl_surface, |state, _, committed| {
            let events =
                session_lock::on_lock_surface_commit(&mut state.wl.session_lock, committed);
            core_loop::feed(state, events);
        });
        compositor::add_destruction_hook::<State, _>(&wl_surface, |state, destroyed| {
            let events =
                session_lock::on_lock_surface_destroyed(&mut state.wl.session_lock, destroyed);
            core_loop::feed(state, events);
        });
        let events = session_lock::on_lock_surface(&mut self.wl.session_lock, surface, key);
        core_loop::feed(self, events);
    }
}
