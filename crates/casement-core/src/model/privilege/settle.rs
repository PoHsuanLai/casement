//! Closing questions: the person's answer, a change in Settings, a deadline, a new floor. And
//! the two things every step ends with: showing the next question, and arming the timer for the
//! next deadline.

use super::ask::{Asker, OpenAsk, Sheet};
use super::book::{Book, Grant, UnixSeconds, Verdict};
use super::decide::{Decision, decide};
use super::edit::Edit;
use super::limits::{ASK_TTL, SHELL_WAIT};
use super::step::{Ctx, PowerOutcome};
use super::subject::Subject;
use super::{Class, Privilege, ShellPresence};
use crate::effect::Effect;
use crate::ids::ClientKey;
use crate::time::{Due, Mono, TimerKey};
use casement_protocols::vocab::{Answer, AskId, Memory, Power};

/// How the clients waiting on a closed question are settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Settle {
    /// An allow that lasts for the connection: a client the book does not let in is let in
    /// once.
    Once,
    /// What the book and the floor say now; a client they leave to a question is refused.
    ByRecord,
}

impl Privilege {
    /// The person answered the question on screen. An answer to a question that is not on
    /// screen (unknown, queued, already closed, timed out) changes nothing.
    pub(super) fn answered(
        &mut self,
        out: &mut Vec<Effect>,
        id: AskId,
        answer: Answer,
        at: UnixSeconds,
        ctx: Ctx,
    ) {
        let Some(ask) = self.asks.take_shown(id) else {
            return;
        };
        let kept = matches!(
            (&ask.key.asker, ask.memory),
            (Asker::Subject(_), Memory::Remembered)
        );
        match (answer, kept) {
            (Answer::Allow, true) => {
                self.remember(out, &ask, Verdict::Allow, at);
                self.settle(out, &ask, Settle::ByRecord);
            }
            (Answer::Deny, true) => {
                self.remember(out, &ask, Verdict::Deny, at);
                self.settle(out, &ask, Settle::ByRecord);
            }
            (Answer::Allow, false) => self.settle(out, &ask, Settle::Once),
            (Answer::Deny, false) | (Answer::Dismissed, _) => {
                self.asks.cool(ask.reask.clone(), ask.key.power, ctx.now);
                self.settle(out, &ask, Settle::ByRecord);
            }
        }
    }

    /// Keeps `verdict` for the program `ask` was about.
    fn remember(
        &mut self,
        out: &mut Vec<Effect>,
        ask: &OpenAsk,
        verdict: Verdict,
        at: UnixSeconds,
    ) {
        if let Asker::Subject(subject) = &ask.key.asker {
            let book = std::mem::take(&mut self.book).with(Grant {
                power: ask.key.power,
                subject: subject.clone(),
                verdict,
                at,
            });
            self.book = book;
            out.push(Effect::SaveBook(Box::new(self.book.clone())));
        }
    }

    /// Resolves every client that waited on the closed question `ask`.
    pub(super) fn settle(&mut self, out: &mut Vec<Effect>, ask: &OpenAsk, how: Settle) {
        let power = ask.key.power;
        for client in &ask.waiting {
            let decision = self
                .clients
                .get(client)
                .map(|record| self.decision_for(&record.class, power));
            let outcome = match (decision, how) {
                (None, _) => continue,
                (Some(Decision::Open), _) => PowerOutcome::Allowed,
                (Some(Decision::Closed), _) | (Some(Decision::Ask(_)), Settle::ByRecord) => {
                    PowerOutcome::Denied
                }
                (Some(Decision::Ask(_)), Settle::Once) => {
                    if let Some(record) = self.clients.get_mut(client) {
                        record.once.insert(power);
                    }
                    PowerOutcome::Allowed
                }
            };
            self.resolve(out, *client, power, outcome);
        }
    }

    /// The person changed their book in Settings: the book is saved if it changed, and every
    /// connection and question follows it.
    pub(super) fn edit(&mut self, out: &mut Vec<Effect>, edit: Edit, at: UnixSeconds) {
        let before = self.book.clone();
        let book = std::mem::take(&mut self.book);
        self.book = match edit {
            Edit::Allow(subject, power) => self.keep(book, subject, power, Verdict::Allow, at),
            Edit::Deny(subject, power) => {
                self.revoke_live(out, &subject, power);
                self.keep(book, subject, power, Verdict::Deny, at)
            }
            Edit::Forget(subject, power) => {
                self.revoke_live(out, &subject, power);
                book.without(&subject, power)
            }
        };
        if self.book != before {
            out.push(Effect::SaveBook(Box::new(self.book.clone())));
        }
        self.reconcile(out);
    }

    /// `book` with the answer, unless the program's answers are never kept.
    fn keep(
        &self,
        book: Book,
        subject: Subject,
        power: Power,
        verdict: Verdict,
        at: UnixSeconds,
    ) -> Book {
        match self.floor.remembers(&subject) {
            Memory::Remembered => book.with(Grant {
                power,
                subject,
                verdict,
                at,
            }),
            Memory::ThisTimeOnly => book,
        }
    }

    /// Every connected client of `subject` loses `power` at once, whether the book says so or
    /// the answer was only for its connection.
    fn revoke_live(&mut self, out: &mut Vec<Effect>, subject: &Subject, power: Power) {
        let held: Vec<ClientKey> = self
            .clients
            .iter()
            .filter(|(_, record)| record.allowed.contains(&power))
            .filter(|(_, record)| match &record.class {
                Class::Public(standing) => standing.subject().as_ref() == Some(subject),
                Class::Trusted | Class::Sandboxed => false,
            })
            .map(|(client, _)| *client)
            .collect();
        for client in held {
            self.resolve(out, client, power, PowerOutcome::Revoked);
        }
    }

    /// The book or the floor changed: a client that had a power and no longer may loses it, and
    /// a question the records now answer is closed.
    pub(super) fn reconcile(&mut self, out: &mut Vec<Effect>) {
        let mut lost: Vec<(ClientKey, Power)> = Vec::new();
        for (client, record) in &self.clients {
            for power in &record.allowed {
                let keeps = match self.decision_for(&record.class, *power) {
                    Decision::Open => true,
                    Decision::Ask(_) => record.once.contains(power),
                    Decision::Closed => false,
                };
                if !keeps {
                    lost.push((*client, *power));
                }
            }
        }
        for (client, power) in lost {
            self.resolve(out, client, power, PowerOutcome::Revoked);
        }
        let answered: Vec<AskId> = self
            .asks
            .open
            .iter()
            .filter(|ask| match &ask.key.asker {
                Asker::Subject(subject) => !matches!(
                    decide(&self.floor, &self.book, Some(subject), ask.key.power),
                    Decision::Ask(_)
                ),
                Asker::Connection(_) => false,
            })
            .map(|ask| ask.id)
            .collect();
        for id in answered {
            if let Some(ask) = self.asks.take(id) {
                if ask.sheet == Sheet::Shown {
                    out.push(Effect::WithdrawAsk { ask: ask.id });
                }
                self.settle(out, &ask, Settle::ByRecord);
            }
        }
    }

    /// When `ask` counts as dismissed: its own deadline, or the end of the wait for a shell.
    fn deadline_of(&self, ask: &OpenAsk) -> Mono {
        match self.shell {
            ShellPresence::Present => ask.deadline,
            ShellPresence::Absent { since } => ask.deadline.min(since.after(SHELL_WAIT)),
        }
    }

    /// Closes the questions whose time is up: withdrawn from the shell, their clients refused,
    /// and, when the person had it in front of them, not asked again for a while.
    pub(super) fn expire(&mut self, out: &mut Vec<Effect>, ctx: Ctx) {
        let due: Vec<AskId> = self
            .asks
            .open
            .iter()
            .filter(|ask| self.deadline_of(ask) <= ctx.now)
            .map(|ask| ask.id)
            .collect();
        for id in due {
            let Some(ask) = self.asks.take(id) else {
                continue;
            };
            if ask.sheet == Sheet::Shown {
                out.push(Effect::WithdrawAsk { ask: ask.id });
                self.asks.cool(ask.reask.clone(), ask.key.power, ctx.now);
            }
            self.settle(out, &ask, Settle::ByRecord);
        }
    }

    /// Shows the next question if the shell can be asked, nothing is on screen and the lock is
    /// not up.
    fn pump(&mut self, out: &mut Vec<Effect>, ctx: Ctx) {
        let ready = !ctx.covered && matches!(self.shell, ShellPresence::Present);
        if !ready || self.asks.is_shown() {
            return;
        }
        if let Some(ask) = self.asks.first_queued_mut() {
            ask.sheet = Sheet::Shown;
            ask.deadline = ctx.now.after(ASK_TTL);
            out.push(Effect::AskPermission {
                ask: ask.id,
                power: ask.key.power,
                subject: ask.subject.clone(),
                memory: ask.memory,
            });
        }
    }

    /// Ends a step: shows the next question, and arms or cancels the timer so the machine is
    /// woken at the next deadline.
    pub(super) fn finish(&mut self, out: &mut Vec<Effect>, ctx: Ctx) {
        self.pump(out, ctx);
        let wanted = self.asks.open.iter().map(|ask| self.deadline_of(ask)).min();
        if wanted == self.timer {
            return;
        }
        out.push(match wanted {
            Some(at) => Effect::ArmTimer(Due {
                at,
                key: TimerKey::AskDeadline,
            }),
            None => Effect::CancelTimer(TimerKey::AskDeadline),
        });
        self.timer = wanted;
    }
}
