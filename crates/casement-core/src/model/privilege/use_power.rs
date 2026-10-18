//! A client used a power: let it, refuse it, or ask the person.
//!
//! The ask has four brakes, so a program cannot make the person's screen busy: the same
//! program and power share one question; a program refused or dismissed lately is not asked
//! again for a while; a client may open only a few different questions a minute; and only a
//! few questions are open at once.

use super::ask::{AskKey, Asker, Join, Reask};
use super::decide::Decision;
use super::limits::{ASKS_PER_MINUTE, RATE_WINDOW, SHELL_WAIT};
use super::step::{Ctx, PowerOutcome, Screen};
use super::{Class, Privilege, ShellPresence};
use crate::effect::Effect;
use crate::ids::ClientKey;
use casement_protocols::vocab::{Memory, Power};

impl Privilege {
    /// `client` used `power`. A client the machine does not know is refused.
    pub(super) fn use_power(
        &mut self,
        out: &mut Vec<Effect>,
        client: ClientKey,
        power: Power,
        ctx: Ctx,
    ) {
        let known = self.clients.get(&client).map(|record| {
            (
                self.decision_for(&record.class, power),
                record.once.contains(&power),
            )
        });
        match known {
            Some((Decision::Open, _) | (Decision::Ask(_), true)) => {
                self.resolve(out, client, power, PowerOutcome::Allowed);
            }
            Some((Decision::Ask(memory), false)) => self.ask(out, client, power, memory, ctx),
            Some((Decision::Closed, _)) | None => {
                self.resolve(out, client, power, PowerOutcome::Denied);
            }
        }
    }

    /// Opens a question for `client` and `power`, or joins the one already open, or refuses.
    fn ask(
        &mut self,
        out: &mut Vec<Effect>,
        client: ClientKey,
        power: Power,
        memory: Memory,
        ctx: Ctx,
    ) {
        let Some(Class::Public(standing)) = self.clients.get(&client).map(|c| c.class.clone())
        else {
            self.resolve(out, client, power, PowerOutcome::Denied);
            return;
        };
        let asker = match standing.subject() {
            Some(subject) => Asker::Subject(subject),
            None => Asker::Connection(client),
        };
        let reask = match (standing.subject(), standing.image_name()) {
            (Some(subject), _) => Reask::Subject(subject),
            (None, Some(name)) => Reask::Image(name.to_owned()),
            (None, None) => Reask::Anonymous,
        };
        let key = AskKey { asker, power };
        let refused = match (ctx.screen, self.shell) {
            (Screen::Covered, _) => true,
            (Screen::Open, ShellPresence::Absent { since }) => ctx.now.since(since) >= SHELL_WAIT,
            (Screen::Open, ShellPresence::Present) => false,
        };
        if refused {
            self.resolve(out, client, power, PowerOutcome::Denied);
            return;
        }
        match self.asks.join(&key, client) {
            Join::Joined => return,
            Join::Full => {
                self.resolve(out, client, power, PowerOutcome::Denied);
                return;
            }
            Join::NoSuchAsk => {}
        }
        let admitted = !self.asks.is_cooling(&reask, power, ctx.now)
            && self.within_rate(client, power, ctx)
            && self.asks.has_room();
        let opened = admitted
            .then(|| {
                self.asks
                    .open_for(key, reask, standing.ask_subject(), memory, client, ctx.now)
            })
            .flatten();
        if opened.is_none() {
            self.resolve(out, client, power, PowerOutcome::Denied);
        }
    }

    /// Whether `client` may open a question for `power`: it asked for fewer than
    /// [`ASKS_PER_MINUTE`] other powers in the last [`RATE_WINDOW`]. A client that may notes
    /// the ask.
    fn within_rate(&mut self, client: ClientKey, power: Power, ctx: Ctx) -> bool {
        let Some(record) = self.clients.get_mut(&client) else {
            return false;
        };
        record
            .recent
            .retain(|_, at| ctx.now.since(*at) < RATE_WINDOW);
        let fits = record.recent.contains_key(&power) || record.recent.len() < ASKS_PER_MINUTE;
        if fits {
            record.recent.insert(power, ctx.now);
        }
        fits
    }
}
