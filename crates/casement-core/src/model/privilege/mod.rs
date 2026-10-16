//! Per-app consent: the seven powers a program must be allowed, remembered per program.
//!
//! ```text
//! use of a power ──► decide(floor, book, who) ──► Open ─────────► Resolve(Allowed)
//!                                             ├─► Closed ───────► Resolve(Denied)
//!                                             └─► Ask ─► one question per (program, power),
//!                                                        one on screen, the answer is kept
//!                                                        in the book or lasts one connection
//! ```
//!
//! Everything here is pure. The composition root reads the two files (the administrator's
//! floor, the person's book), tells the model who each public client is
//! ([`ClientStanding`], filled from the caller-identity library), forwards the first use of a
//! power, shows the question through the shell and writes the book when the model says so.
//! Core never reads a process, a file or a clock.
//!
//! The machine ([`Privilege`]) lives inside [`Model`](crate::Model), which hands it the events
//! it understands (`step.rs`) and the effects it answers with are the model's own
//! ([`Effect::AskPermission`](crate::Effect) and its siblings).

mod ask;
mod book;
mod book_codec;
mod decide;
mod floor;
mod floor_codec;
mod limits;
mod route;
mod rows;
mod settle;
mod standing;
mod step;
mod subject;
mod subject_dto;
mod use_power;

pub use ask::{AskView, Sheet};
pub use book::{Book, Grant, UnixSeconds, Verdict};
pub use book_codec::{BookError, Loaded};
pub use decide::{Decision, decide, decide_trusted};
pub use floor::{Fallback, Floor, FloorRule};
pub use floor_codec::FloorError;
pub use limits::{
    ASK_TTL, ASKS_PER_MINUTE, HELD_REQUESTS, MAX_FILE_BYTES, MAX_GRANTS, OPEN_ASKS, RATE_WINDOW,
    REASK_AFTER, SHELL_WAIT,
};
pub use rows::{PrivacyRow, Source};
pub use standing::{AskSubject, ClientStanding, Identity, ServiceRole};
pub use step::PowerOutcome;
pub use subject::{Engine, ProgramPath, SandboxAppId, Subject};

pub(crate) use step::{Ctx, Input, Screen};

use crate::ids::ClientKey;
use crate::policy::{ConnectionRole, PowerSet, trusted_powers};
use crate::time::Mono;
use ask::Asks;
use casement_protocols::vocab::{Power, WireCode};
use std::collections::{BTreeMap, BTreeSet};

/// What a connection is, as far as consent goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Class {
    /// The shell's or the agent tier's connection: handed out by the compositor, trusted.
    Trusted,
    /// A sandbox's connection: it is not shown the privileged globals at all.
    Sandboxed,
    /// A program on the public socket, as well as it is known.
    Public(ClientStanding),
}

impl Class {
    /// The class of a connection that arrived through a socket of `role`; a public client is
    /// unproven until [`Event::ClientStanding`](crate::Event) says more.
    pub(crate) fn of(role: &ConnectionRole) -> Class {
        match role {
            ConnectionRole::Shell | ConnectionRole::Agent => Class::Trusted,
            ConnectionRole::Sandboxed(_) => Class::Sandboxed,
            ConnectionRole::Public => Class::Public(ClientStanding::unproven()),
        }
    }
}

/// One connected client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Client {
    class: Class,
    /// The powers resolved as allowed to this connection, which the protocol layer has cached.
    allowed: BTreeSet<Power>,
    /// Of those, the ones allowed for this connection only (Allow Once), not kept in the book.
    once: BTreeSet<Power>,
    /// When this client last opened a question for each power, for the rate limit.
    recent: BTreeMap<Power, Mono>,
}

impl Client {
    fn new(class: Class) -> Client {
        Client {
            class,
            allowed: BTreeSet::new(),
            once: BTreeSet::new(),
            recent: BTreeMap::new(),
        }
    }
}

/// Whether a shell can be asked to show a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellPresence {
    /// Its protocol is bound.
    Present,
    /// It is not (yet, or any more) since this instant.
    Absent { since: Mono },
}

/// The consent machine's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Privilege {
    floor: Floor,
    book: Book,
    clients: BTreeMap<ClientKey, Client>,
    asks: Asks,
    shell: ShellPresence,
    /// When the timer that wakes the machine for the next deadline is armed for.
    timer: Option<Mono>,
}

impl Privilege {
    /// A machine at `epoch` with no floor, an empty book and no shell: a question waits
    /// [`SHELL_WAIT`] for the shell to start.
    pub(crate) fn new(epoch: Mono) -> Privilege {
        Privilege {
            floor: Floor::empty(),
            book: Book::empty(),
            clients: BTreeMap::new(),
            asks: Asks::new(),
            shell: ShellPresence::Absent { since: epoch },
            timer: None,
        }
    }

    /// The person's book as it is now.
    pub fn book(&self) -> &Book {
        &self.book
    }

    /// The administrator's floor as it is now.
    pub fn floor(&self) -> &Floor {
        &self.floor
    }

    /// The questions that are open.
    pub fn asks(&self) -> Vec<AskView> {
        self.asks.views()
    }

    /// The powers a public client whose program is `standing` is refused outright: the
    /// globals of those are not advertised to it. Nothing for any other kind of connection.
    pub fn hidden_for(&self, role: &ConnectionRole, standing: &ClientStanding) -> PowerSet {
        match role {
            ConnectionRole::Public => {
                let class = Class::Public(standing.clone());
                <Power as WireCode>::ALL
                    .iter()
                    .copied()
                    .filter(|power| self.decision_for(&class, *power) == Decision::Closed)
                    .collect()
            }
            ConnectionRole::Shell | ConnectionRole::Agent | ConnectionRole::Sandboxed(_) => {
                PowerSet::default()
            }
        }
    }

    /// What a connection of `class` gets for `power` now.
    fn decision_for(&self, class: &Class, power: Power) -> Decision {
        match class {
            Class::Trusted => Decision::Open,
            Class::Sandboxed => Decision::Closed,
            Class::Public(standing) => {
                let identity = standing.identity();
                let trusted = standing
                    .service_role()
                    .map(trusted_powers)
                    .unwrap_or_default();
                match trusted.contains(&power) {
                    true => decide_trusted(&self.floor, &self.book, &identity, power),
                    false => decide(&self.floor, &self.book, &identity, power),
                }
            }
        }
    }

    /// Whether the machine's books agree with each other: at most [`OPEN_ASKS`] questions, at
    /// most one on screen, every id once, every waiting client connected, every one-time allow
    /// an allow, and the book within its cap.
    pub(crate) fn in_step(&self) -> bool {
        let ids: BTreeSet<_> = self.asks.open.iter().map(|ask| ask.id).collect();
        let shown = self
            .asks
            .open
            .iter()
            .filter(|ask| ask.sheet == Sheet::Shown)
            .count();
        self.asks.open.len() <= OPEN_ASKS
            && shown <= 1
            && ids.len() == self.asks.open.len()
            && self
                .asks
                .open
                .iter()
                .all(|ask| ask.waiting.iter().all(|c| self.clients.contains_key(c)))
            && self
                .clients
                .values()
                .all(|client| client.once.is_subset(&client.allowed))
            && self.book.len() <= MAX_GRANTS
    }
}
