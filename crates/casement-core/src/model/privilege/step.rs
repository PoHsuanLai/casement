//! The consent machine's one transition: an [`Input`] and the instant it happens, in; the
//! next machine and the effects it asks for, out.

use super::book::{Book, UnixSeconds};
use super::floor::Floor;
use super::standing::ClientStanding;
use super::subject::Subject;
use super::{Class, Client, Privilege, ShellPresence};
use crate::effect::Effect;
use crate::ids::ClientKey;
use crate::time::Mono;
use casement_protocols::vocab::{Answer, AskId, Power};

/// What the protocol layer is told about a use of a power.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PowerOutcome {
    /// The client may use the power: replay what was held.
    Allowed,
    /// It may not: give it the protocol's denial and drop what was held. Not cached for good
    /// when the answer was not kept.
    Denied,
    /// It could and now cannot (the person switched it off, the floor changed): end what is
    /// in use at once.
    Revoked,
}

/// Whether the screen is covered by the lock, when nobody can be asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Screen {
    /// The person can see a question.
    Open,
    /// The lock is up or coming up: no question is shown, new uses are refused.
    Covered,
}

/// What the machine is told besides the event: the instant, and the lock.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Ctx {
    pub(crate) now: Mono,
    pub(crate) screen: Screen,
}

/// What can happen to the machine.
#[derive(Debug, Clone)]
pub(crate) enum Input {
    /// A client connected through a socket of this class.
    Connected { client: ClientKey, class: Class },
    /// The root says who a public client is.
    Standing {
        client: ClientKey,
        standing: ClientStanding,
    },
    /// A client went.
    Gone { client: ClientKey },
    /// A client used a power for the first time (or again after a refusal).
    Use { client: ClientKey, power: Power },
    /// The person answered a question on screen.
    Answered {
        ask: AskId,
        answer: Answer,
        at: UnixSeconds,
    },
    /// Settings switched a program's power on.
    Granted {
        subject: Subject,
        power: Power,
        at: UnixSeconds,
    },
    /// Settings switched a program's power off.
    Revoked {
        subject: Subject,
        power: Power,
        at: UnixSeconds,
    },
    /// Settings forgot a program's answer: it is asked again.
    Forgotten { subject: Subject, power: Power },
    /// The shell can be asked.
    ShellReady,
    /// The shell cannot be asked any more.
    ShellGone,
    /// The book was read at start.
    BookLoaded(Book),
    /// The floor was read at start or reloaded.
    FloorLoaded(Floor),
    /// The deadline timer fired.
    Tick,
}

/// The machine after an input, and what it asks for.
#[derive(Debug)]
pub(crate) struct Stepped {
    pub(crate) privilege: Privilege,
    pub(crate) effects: Vec<Effect>,
}

impl Privilege {
    /// The machine after `input` at `ctx.now`, and the effects it asks for, in order. Deadlines
    /// that passed are acted on first, so a late answer finds its question gone.
    pub(crate) fn step(self, input: Input, ctx: Ctx) -> Stepped {
        let mut next = self;
        let mut out = Vec::new();
        next.expire(&mut out, ctx);
        match input {
            Input::Connected { client, class } => {
                next.clients.insert(client, Client::new(class));
            }
            Input::Standing { client, standing } => next.stand(&mut out, client, standing),
            Input::Gone { client } => next.gone(client),
            Input::Use { client, power } => next.use_power(&mut out, client, power, ctx),
            Input::Answered { ask, answer, at } => next.answered(&mut out, ask, answer, at, ctx),
            Input::Granted { subject, power, at } => {
                next.edit(&mut out, super::settle::Edit::Allow(subject, power, at));
            }
            Input::Revoked { subject, power, at } => {
                next.edit(&mut out, super::settle::Edit::Deny(subject, power, at));
            }
            Input::Forgotten { subject, power } => {
                next.edit(&mut out, super::settle::Edit::Forget(subject, power));
            }
            Input::ShellReady => next.shell = ShellPresence::Present,
            Input::ShellGone => next.shell_gone(ctx),
            Input::BookLoaded(book) => {
                next.book = book;
                next.reconcile(&mut out);
            }
            Input::FloorLoaded(floor) => {
                next.floor = floor;
                next.reconcile(&mut out);
            }
            Input::Tick => next.timer = None,
        }
        next.finish(&mut out, ctx);
        Stepped {
            privilege: next,
            effects: out,
        }
    }

    /// The root says who a public client is. A client that becomes someone else loses what it
    /// was given and what it waited for: the answers were for the program it was.
    fn stand(&mut self, out: &mut Vec<Effect>, client: ClientKey, standing: ClientStanding) {
        let Some(record) = self.clients.get_mut(&client) else {
            return;
        };
        match &record.class {
            Class::Public(old) if *old != standing => {}
            Class::Public(_) | Class::Trusted | Class::Sandboxed => return,
        }
        let revoked: Vec<Power> = record.allowed.iter().copied().collect();
        record.class = Class::Public(standing);
        for power in revoked {
            self.resolve(out, client, power, PowerOutcome::Revoked);
        }
        for power in self.asks.leave(client) {
            self.resolve(out, client, power, PowerOutcome::Denied);
        }
    }

    /// A client went: it waits on nothing, its questions stay open for the next time.
    fn gone(&mut self, client: ClientKey) {
        self.clients.remove(&client);
        self.asks.leave(client);
    }

    /// The shell cannot be asked: what was on its screen waits for the next shell.
    fn shell_gone(&mut self, ctx: Ctx) {
        if self.shell == ShellPresence::Present {
            self.shell = ShellPresence::Absent { since: ctx.now };
        }
        self.asks.requeue();
    }

    /// Tells the protocol layer how a use of `power` by `client` came out, and keeps the
    /// machine's idea of what the client may do in step with it.
    pub(super) fn resolve(
        &mut self,
        out: &mut Vec<Effect>,
        client: ClientKey,
        power: Power,
        outcome: PowerOutcome,
    ) {
        if let Some(record) = self.clients.get_mut(&client) {
            match outcome {
                PowerOutcome::Allowed => {
                    record.allowed.insert(power);
                }
                PowerOutcome::Denied | PowerOutcome::Revoked => {
                    record.allowed.remove(&power);
                    record.once.remove(&power);
                }
            }
        }
        out.push(Effect::Resolve {
            client,
            power,
            outcome,
        });
    }
}
