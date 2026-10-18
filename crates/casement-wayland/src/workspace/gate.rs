//! The consent gate of `ext_workspace_manager_v1` (`Power::Windows`): binding it lists the
//! Spaces and lets the client switch, move and remove them, so a manager bound while the person
//! has not answered hears nothing; refused, it says `finished`. A power taken back finishes the
//! managers in use, and what they had queued is dropped.

use super::{Pending, WorkspaceHandler, WorkspacePart, attach};
use crate::consent::{self, Mortal, Pushed};
use casement_core::ClientKey;
use casement_core::privilege::PowerOutcome;
use smithay::reexports::wayland_protocols::ext::workspace::v1::server::ext_workspace_manager_v1::ExtWorkspaceManagerV1;
use smithay::reexports::wayland_server::{Client, Resource};
use std::sync::PoisonError;

/// A manager that waits for the person's answer.
#[derive(Debug)]
pub struct WaitingManager {
    manager: ExtWorkspaceManagerV1,
    pending: Pending,
}

impl WaitingManager {
    /// The manager `manager`, whose queued requests are `pending`.
    pub(super) fn new(manager: ExtWorkspaceManagerV1, pending: Pending) -> WaitingManager {
        WaitingManager { manager, pending }
    }
}

impl Mortal for WaitingManager {
    fn alive(&self) -> bool {
        self.manager.is_alive()
    }
}

/// Keeps `waiting` until the answer comes; one past the cap of waiting requests, and one from a
/// client nobody vouches for, is finished.
pub(super) fn wait<D: WorkspaceHandler>(state: &mut D, client: &Client, waiting: WaitingManager) {
    let refused = match consent::key_of(client) {
        Some(key) => match state.workspace_part().waiting.push(key, waiting) {
            Pushed::Kept => return,
            Pushed::Full(full) => full,
        },
        None => waiting,
    };
    consent::finish_later(consent::Farewell::Workspaces(refused.manager));
}

/// The client a manager belongs to, when casement inserted it.
fn owner_of(manager: &ExtWorkspaceManagerV1) -> Option<ClientKey> {
    manager.client().as_ref().and_then(consent::key_of)
}

/// The answer about `client`'s power over the windows came out. Allowed, the managers that
/// waited are told the Spaces; refused, they are finished; taken back, so are the ones in use.
pub fn on_resolved(part: &mut WorkspacePart, client: ClientKey, outcome: PowerOutcome) {
    let waiting = part.waiting.take(client);
    match outcome {
        PowerOutcome::Allowed => {
            for held in waiting.into_iter().filter(|held| held.manager.is_alive()) {
                attach(part, held.manager, held.pending);
            }
        }
        PowerOutcome::Denied | PowerOutcome::Revoked => {
            for held in &waiting {
                held.manager.finished();
            }
            let (ended, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut part.bound)
                .into_iter()
                .partition(|bound| owner_of(&bound.manager) == Some(client));
            part.bound = kept;
            for bound in ended {
                bound
                    .pending
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clear();
                bound.manager.finished();
            }
        }
    }
}
