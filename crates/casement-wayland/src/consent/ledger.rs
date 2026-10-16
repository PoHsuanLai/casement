//! The latest word on each client's use of each power: the cache the protocol modules read
//! at every use, and the model's `Effect::Resolve` writes.

use casement_core::ClientKey;
use casement_core::privilege::PowerOutcome;
use casement_protocols::vocab::Power;
use smithay::reexports::wayland_server::backend::ClientId;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// What is known of one client's use of one power.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Word {
    /// The model was asked and has not answered yet: requests wait.
    Waiting,
    /// The client may use the power.
    Allowed,
    /// It may not (refused, or taken back). The next use asks the model again.
    Refused,
}

/// What a protocol module does with a request that needs a power.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// The power is allowed: carry the request out.
    Go,
    /// The answer is not in: keep the request (`Held`) until `Effect::Resolve`.
    Hold,
    /// The power is refused: give the protocol's own denial.
    Refuse,
}

/// How a first look at a client's power came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Begin {
    /// The ledger already knows enough to decide.
    Settled(Admission),
    /// Nothing was known (or the last word was a refusal): the model must be asked.
    Fresh,
}

/// The latest word per client and power.
type Rows = BTreeMap<(ClientKey, Power), Word>;

/// Every client's latest word per power. Shared by clone: the display's client data forgets a
/// client's rows when it disconnects, the compositor root writes the model's answers.
///
/// It also remembers the connection of each client that has objects on hold in a protocol
/// smithay serves whole (the clipboard managers, the input method, the window list of
/// `ext_foreign_toplevel_list`): smithay settles what it holds by connection, the model answers
/// by client key.
///
/// It also knows the connection of every client casement inserted ([`Ledger::connection`]), so a
/// power taken back can end the objects smithay registered for a client that was allowed at once
/// and never had anything on hold.
#[derive(Debug, Clone, Default)]
pub struct Ledger {
    rows: Arc<Mutex<Rows>>,
    parked: Arc<Mutex<BTreeMap<ClientKey, ClientId>>>,
    connections: Arc<Mutex<BTreeMap<ClientKey, ClientId>>>,
}

impl PartialEq for Ledger {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.rows, &other.rows)
    }
}

impl Eq for Ledger {}

impl Ledger {
    /// A poisoned lock is still a lock: the rows are plain data.
    fn rows(&self) -> MutexGuard<'_, Rows> {
        self.rows.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn parked_ids(&self) -> MutexGuard<'_, BTreeMap<ClientKey, ClientId>> {
        self.parked.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn connection_ids(&self) -> MutexGuard<'_, BTreeMap<ClientKey, ClientId>> {
        self.connections
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// `client` was inserted as connection `id`.
    pub fn connected(&self, client: ClientKey, id: ClientId) {
        self.connection_ids().insert(client, id);
    }

    /// The connection of `client`, while it is connected.
    pub fn connection(&self, client: ClientKey) -> Option<ClientId> {
        self.connection_ids().get(&client).cloned()
    }

    /// `client` (connection `id`) has objects on hold in a protocol smithay serves whole.
    pub fn park(&self, client: ClientKey, id: ClientId) {
        self.parked_ids().insert(client, id);
    }

    /// How many clients have a connection parked.
    pub fn parked_count(&self) -> usize {
        self.parked_ids().len()
    }

    /// The connection of `client`, when it has had objects on hold since it connected.
    pub fn parked(&self, client: ClientKey) -> Option<ClientId> {
        self.parked_ids().get(&client).cloned()
    }

    /// The latest word on `client` and `power`, if any.
    pub fn word(&self, client: ClientKey, power: Power) -> Option<Word> {
        self.rows().get(&(client, power)).copied()
    }

    /// What a request of `client` for `power` does now.
    pub fn admission(&self, client: ClientKey, power: Power) -> Admission {
        match self.word(client, power) {
            Some(Word::Allowed) => Admission::Go,
            Some(Word::Waiting) => Admission::Hold,
            Some(Word::Refused) | None => Admission::Refuse,
        }
    }

    /// A first look: an allowed or waiting power is settled; anything else becomes waiting and
    /// the model is to be asked (a refusal is asked again, since the model decides whether the
    /// refusal still stands).
    pub(super) fn begin(&self, client: ClientKey, power: Power) -> Begin {
        let mut rows = self.rows();
        match rows.get(&(client, power)).copied() {
            Some(Word::Allowed) => Begin::Settled(Admission::Go),
            Some(Word::Waiting) => Begin::Settled(Admission::Hold),
            Some(Word::Refused) | None => {
                rows.insert((client, power), Word::Waiting);
                Begin::Fresh
            }
        }
    }

    /// The model's answer: the word it makes, which is also returned.
    pub fn resolve(&self, client: ClientKey, power: Power, outcome: PowerOutcome) -> Word {
        let word = match outcome {
            PowerOutcome::Allowed => Word::Allowed,
            PowerOutcome::Denied | PowerOutcome::Revoked => Word::Refused,
        };
        self.rows().insert((client, power), word);
        word
    }

    /// `client` disconnected: its rows, its connection and its parked connection go.
    pub fn forget(&self, client: ClientKey) {
        self.rows().retain(|(owner, _), _| *owner != client);
        self.parked_ids().remove(&client);
        self.connection_ids().remove(&client);
    }

    /// How many rows there are (a client has at most one per power).
    pub fn len(&self) -> usize {
        self.rows().len()
    }

    /// Whether there are no rows.
    pub fn is_empty(&self) -> bool {
        self.rows().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: ClientKey = ClientKey(1);

    #[test]
    fn a_first_use_waits_and_the_answer_settles_it() {
        let ledger = Ledger::default();
        assert_eq!(ledger.begin(ME, Power::Screen), Begin::Fresh);
        assert_eq!(ledger.admission(ME, Power::Screen), Admission::Hold);
        assert_eq!(
            ledger.begin(ME, Power::Screen),
            Begin::Settled(Admission::Hold)
        );
        ledger.resolve(ME, Power::Screen, PowerOutcome::Allowed);
        assert_eq!(ledger.admission(ME, Power::Screen), Admission::Go);
        assert_eq!(ledger.admission(ME, Power::Keys), Admission::Refuse);
    }

    #[test]
    fn a_refusal_is_asked_again_and_a_revocation_is_a_refusal() {
        let ledger = Ledger::default();
        ledger.resolve(ME, Power::Keys, PowerOutcome::Allowed);
        ledger.resolve(ME, Power::Keys, PowerOutcome::Revoked);
        assert_eq!(ledger.word(ME, Power::Keys), Some(Word::Refused));
        assert_eq!(ledger.begin(ME, Power::Keys), Begin::Fresh);
        assert_eq!(ledger.word(ME, Power::Keys), Some(Word::Waiting));
    }

    #[test]
    fn a_disconnect_forgets_only_that_client() {
        let ledger = Ledger::default();
        ledger.resolve(ME, Power::Lock, PowerOutcome::Allowed);
        ledger.resolve(ClientKey(2), Power::Lock, PowerOutcome::Allowed);
        ledger.forget(ME);
        assert_eq!(ledger.len(), 1);
        assert_eq!(ledger.word(ME, Power::Lock), None);
    }
}
