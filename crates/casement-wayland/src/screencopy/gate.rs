//! The consent gate of `zwlr_screencopy_manager_v1` (`Power::Screen`): a copy asked for while
//! the answer is out waits here, and `failed` is the denial. A copy already queued when the
//! power is taken back fails with it.

use super::{PendingCopy, ScreencopyPart, apply_failed};
use crate::consent::{self, Held, Mortal, Pushed};
use casement_core::ClientKey;
use casement_core::privilege::PowerOutcome;
use smithay::reexports::wayland_server::Resource;

impl Mortal for PendingCopy {
    fn alive(&self) -> bool {
        self.frame.is_alive()
    }
}

/// The copies that wait for the person, by client.
pub type WaitingCopies = Held<PendingCopy>;

/// The client a copy belongs to, when casement inserted it.
fn owner_of(copy: &PendingCopy) -> Option<ClientKey> {
    copy.frame.client().as_ref().and_then(consent::key_of)
}

impl ScreencopyPart {
    /// Keeps `copy` until the answer about its client's screen comes out; a client with
    /// [`HELD_REQUESTS`](casement_core::privilege::HELD_REQUESTS) waiting already, and one
    /// nobody vouches for, gets `failed`.
    pub fn hold(&mut self, copy: PendingCopy) {
        match owner_of(&copy) {
            Some(client) => {
                if let Pushed::Full(copy) = self.waiting.push(client, copy) {
                    apply_failed(&copy);
                }
            }
            None => apply_failed(&copy),
        }
    }

    /// How many copies `client` has waiting for an answer.
    pub fn held_of(&self, client: ClientKey) -> usize {
        self.waiting.count(client)
    }
}

/// The answer about `client`'s screen came out. Allowed, its waiting copies are queued (the
/// caller answers them); refused or taken back, they fail, and so do the ones already queued.
pub fn on_resolved(part: &mut ScreencopyPart, client: ClientKey, outcome: PowerOutcome) {
    let waiting = part.waiting.take(client);
    match outcome {
        PowerOutcome::Allowed => waiting.into_iter().for_each(|copy| part.keep(copy)),
        PowerOutcome::Denied | PowerOutcome::Revoked => {
            waiting.iter().for_each(apply_failed);
            let (ended, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut part.pending)
                .into_iter()
                .partition(|copy| owner_of(copy) == Some(client));
            part.pending = kept;
            ended.iter().for_each(apply_failed);
        }
    }
}
