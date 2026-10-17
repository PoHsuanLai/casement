//! The questions to the person: which are open, which is on screen, who waits on each, and
//! which programs are not asked again for a while.

use super::limits::{ASK_TTL, MAX_COOLDOWNS, MAX_WAITERS, OPEN_ASKS, REASK_AFTER};
use super::standing::AskSubject;
use super::subject::Subject;
use crate::ids::ClientKey;
use crate::time::Mono;
use casement_protocols::vocab::{AskId, Memory, Power};
use std::collections::{BTreeMap, BTreeSet};

/// Who a question is about: a program the answer can be kept for, or one connection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Asker {
    /// A program or app; two clients of it share one question.
    Subject(Subject),
    /// A program that cannot be named; its question and its answer are its connection's.
    Connection(ClientKey),
}

/// Whom the pause before asking again belongs to (NEW-CAS-5). A question belongs to its
/// connection when the program cannot be named, but the pause must not: a program that
/// reconnects after "Don't Allow" would be asked again at once, forever.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reask {
    /// A program or app that can be named.
    Subject(Subject),
    /// A program that cannot be named but whose image has a file name: every program of that
    /// name shares the pause.
    Image(String),
    /// A program that cannot be told from any other unnamed one: they all share one pause.
    Anonymous,
}

/// What a question is for: one asker and one power.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AskKey {
    /// Who.
    pub asker: Asker,
    /// Which power.
    pub power: Power,
}

/// Whether the shell has been told of a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sheet {
    /// Waiting for its turn, or for a shell.
    Queued,
    /// On the person's screen.
    Shown,
}

/// One open question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAsk {
    /// Its id on the wire.
    pub id: AskId,
    /// What it is for.
    pub key: AskKey,
    /// Whom a dismissal pauses further questions for.
    pub reask: Reask,
    /// What the shell is told of the program.
    pub subject: AskSubject,
    /// Whether an answer is kept.
    pub memory: Memory,
    /// Whether the shell has it.
    pub sheet: Sheet,
    /// When it counts as dismissed.
    pub deadline: Mono,
    /// The clients whose requests wait on it.
    pub waiting: BTreeSet<ClientKey>,
}

/// A question as the outside sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskView {
    /// Its id.
    pub id: AskId,
    /// Which power.
    pub power: Power,
    /// Whether the shell has it.
    pub sheet: Sheet,
    /// How many clients wait on it.
    pub waiting: usize,
}

/// Every open question and every pause before asking again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asks {
    next: Option<u32>,
    pub(super) open: Vec<OpenAsk>,
    cooling: BTreeMap<(Reask, Power), Mono>,
}

impl Default for Asks {
    fn default() -> Asks {
        Asks::new()
    }
}

impl Asks {
    /// No questions; the first id is 1.
    pub fn new() -> Asks {
        Asks {
            next: Some(1),
            open: Vec::new(),
            cooling: BTreeMap::new(),
        }
    }

    /// The next id, or none when the numbers are spent (an id is never reused).
    fn mint(&mut self) -> Option<AskId> {
        let id = self.next?;
        self.next = id.checked_add(1);
        Some(AskId::new(id))
    }

    /// Whether another question fits.
    pub(super) fn has_room(&self) -> bool {
        self.open.len() < OPEN_ASKS
    }

    /// The question for `key`, if one is open.
    pub(super) fn find_mut(&mut self, key: &AskKey) -> Option<&mut OpenAsk> {
        self.open.iter_mut().find(|ask| ask.key == *key)
    }

    /// `client` waits on the open question for `key`. Whether it was joined, and if not, why.
    pub(super) fn join(&mut self, key: &AskKey, client: ClientKey) -> Join {
        match self.find_mut(key) {
            None => Join::NoSuchAsk,
            Some(ask) if ask.waiting.len() >= MAX_WAITERS && !ask.waiting.contains(&client) => {
                Join::Full
            }
            Some(ask) => {
                ask.waiting.insert(client);
                Join::Joined
            }
        }
    }

    /// Opens a queued question for `key` with `client` waiting, unless the ids are spent.
    pub(super) fn open_for(
        &mut self,
        key: AskKey,
        reask: Reask,
        subject: AskSubject,
        memory: Memory,
        client: ClientKey,
        now: Mono,
    ) -> Option<AskId> {
        let id = self.mint()?;
        self.open.push(OpenAsk {
            id,
            key,
            reask,
            subject,
            memory,
            sheet: Sheet::Queued,
            deadline: now.after(ASK_TTL),
            waiting: BTreeSet::from([client]),
        });
        Some(id)
    }

    /// The open question `id`, taken out, if it is on screen.
    pub(super) fn take_shown(&mut self, id: AskId) -> Option<OpenAsk> {
        let at = self
            .open
            .iter()
            .position(|ask| ask.id == id && ask.sheet == Sheet::Shown)?;
        Some(self.open.remove(at))
    }

    /// The open question `id`, taken out.
    pub(super) fn take(&mut self, id: AskId) -> Option<OpenAsk> {
        let at = self.open.iter().position(|ask| ask.id == id)?;
        Some(self.open.remove(at))
    }

    /// Whether a question is on screen.
    pub(super) fn is_shown(&self) -> bool {
        self.open.iter().any(|ask| ask.sheet == Sheet::Shown)
    }

    /// The first queued question.
    pub(super) fn first_queued_mut(&mut self) -> Option<&mut OpenAsk> {
        self.open.iter_mut().find(|ask| ask.sheet == Sheet::Queued)
    }

    /// `client` no longer waits on anything; the powers it was waiting for.
    pub(super) fn leave(&mut self, client: ClientKey) -> Vec<Power> {
        self.open
            .iter_mut()
            .filter_map(|ask| ask.waiting.remove(&client).then_some(ask.key.power))
            .collect()
    }

    /// Nothing is on screen any more (the shell went): every question waits again.
    pub(super) fn requeue(&mut self) {
        self.open
            .iter_mut()
            .for_each(|ask| ask.sheet = Sheet::Queued);
    }

    /// `who` is not asked about `power` again before [`REASK_AFTER`] from `now`. The table is
    /// bounded: the entry that ends soonest makes room.
    pub(super) fn cool(&mut self, who: Reask, power: Power, now: Mono) {
        self.cooling.retain(|_, until| *until > now);
        self.cooling.insert((who, power), now.after(REASK_AFTER));
        while self.cooling.len() > MAX_COOLDOWNS {
            let soonest = self
                .cooling
                .iter()
                .min_by_key(|(_, until)| **until)
                .map(|(key, _)| key.clone());
            match soonest {
                Some(soonest) => self.cooling.remove(&soonest),
                None => break,
            };
        }
    }

    /// Whether `who` was dismissed about `power` lately and is not asked again yet.
    pub(super) fn is_cooling(&self, who: &Reask, power: Power, now: Mono) -> bool {
        self.cooling
            .get(&(who.clone(), power))
            .is_some_and(|until| *until > now)
    }

    /// The questions as the outside sees them, in the order they were opened.
    pub fn views(&self) -> Vec<AskView> {
        self.open
            .iter()
            .map(|ask| AskView {
                id: ask.id,
                power: ask.key.power,
                sheet: ask.sheet,
                waiting: ask.waiting.len(),
            })
            .collect()
    }
}

/// What joining an open question came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Join {
    /// The client waits on it now.
    Joined,
    /// No such question is open.
    NoSuchAsk,
    /// The question has as many waiting clients as one may.
    Full,
}
