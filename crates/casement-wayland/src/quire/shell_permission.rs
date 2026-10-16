//! `quire_shell_v1` version 5: the compositor asks the person, through the shell, whether a
//! program may use a power (`permission_ask`), takes a question back (`permission_withdraw`)
//! and hears the answer (`permission_answer`). The model decides what to ask and when
//! (`casement_core::privilege`); this only carries the words.
//!
//! The part keeps the questions it has sent and not seen answered or withdrawn, so a second
//! object of the shell that binds while one is on screen is shown it too. A shell that bound a
//! version older than 5 has nothing to answer with and is told nothing; the model is told the
//! shell can be asked only once an object of version 5 or later is bound, and that it cannot
//! when the last one goes.

use super::code_of;
use super::shell::QuireShellPart;
use casement_core::Event;
use casement_core::privilege::{AskSubject, UnixSeconds};
use casement_protocols::server::quire_shell_v1::quire_shell_v1::{self, QuireShellV1};
use casement_protocols::vocab::{Answer, AskId, Memory, Power, WireCode};
use smithay::reexports::wayland_server::{Resource, WEnum};
use std::fmt::Debug;

/// The first version of `quire_shell_v1` that carries the question.
pub const SINCE: u32 = 5;

/// The most questions the part remembers having sent: the model keeps four open at once
/// ([`OPEN_ASKS`](casement_core::privilege::OPEN_ASKS)); the bound is only a guard.
const MAX_SENT: usize = 16;

/// What the root knows of the date, in whole seconds: the stamp on an answer that is kept. Core
/// reads no clock; the root replaces the part's [`UnknownWall`] with a real one.
pub trait WallSeconds: Debug {
    /// Seconds since the Unix epoch.
    fn now(&self) -> UnixSeconds;
}

/// A date nobody told the part: every answer is stamped with the epoch.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnknownWall;

impl WallSeconds for UnknownWall {
    fn now(&self) -> UnixSeconds {
        UnixSeconds(0)
    }
}

/// A question that was sent to the shell and is not settled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentAsk {
    ask: AskId,
    power: Power,
    subject: AskSubject,
    memory: Memory,
}

/// Sends `sent` to `shell`, when the object is new enough to carry it.
fn send(shell: &QuireShellV1, sent: &SentAsk) {
    if shell.version() < SINCE {
        return;
    }
    let wire = (
        quire_shell_v1::Power::try_from(sent.power.code()),
        quire_shell_v1::SubjectKind::try_from(sent.subject.kind.code()),
        quire_shell_v1::Memory::try_from(sent.memory.code()),
    );
    if let (Ok(power), Ok(kind), Ok(memory)) = wire {
        shell.permission_ask(
            sent.ask.get(),
            power,
            kind,
            sent.subject.text.clone(),
            memory,
        );
    }
}

/// `Effect::AskPermission`: puts the question to every bound shell object that can carry it.
pub fn apply_ask(
    part: &mut QuireShellPart,
    ask: AskId,
    power: Power,
    subject: &AskSubject,
    memory: Memory,
) {
    let sent = SentAsk {
        ask,
        power,
        subject: subject.clone(),
        memory,
    };
    part.bound.iter().for_each(|shell| send(shell, &sent));
    part.asks.retain(|old| old.ask != ask);
    if part.asks.len() >= MAX_SENT {
        part.asks.remove(0);
    }
    part.asks.push(sent);
}

/// `Effect::WithdrawAsk`: takes the question back from every bound shell object.
pub fn apply_withdraw(part: &mut QuireShellPart, ask: AskId) {
    part.asks.retain(|sent| sent.ask != ask);
    for shell in &part.bound {
        if shell.version() >= SINCE {
            shell.permission_withdraw(ask.get());
        }
    }
}

/// `permission_answer`: the person's word on `ask`. An answer for a question the part has not
/// sent or has settled, and one with a code the XML does not name, changes nothing.
pub fn on_answer(
    part: &mut QuireShellPart,
    ask: u32,
    answer: WEnum<quire_shell_v1::Answer>,
) -> Vec<Event> {
    let Some(at) = part.asks.iter().position(|sent| sent.ask.get() == ask) else {
        return Vec::new();
    };
    let Some(answer) = code_of(answer).and_then(|code| Answer::from_code(code).ok()) else {
        return Vec::new();
    };
    let sent = part.asks.remove(at);
    vec![Event::PermissionAnswered {
        ask: sent.ask,
        answer,
        at: part.wall.now(),
    }]
}

/// A shell object bound: it is shown the questions still open, and when it is the first that
/// can carry them the model hears the shell can be asked (it then sends the question the
/// shell lost, if one was waiting).
pub fn on_bind(part: &QuireShellPart, shell: &QuireShellV1) -> Vec<Event> {
    if shell.version() < SINCE {
        return Vec::new();
    }
    part.asks.iter().for_each(|sent| send(shell, sent));
    vec![Event::ShellReady]
}

/// A shell object of `version` went. When it was the last that could carry the question, the
/// part forgets the questions it sent (the model sends them again to the next shell) and the
/// model hears the shell cannot be asked.
pub fn on_unbind(part: &mut QuireShellPart, version: u32) -> Vec<Event> {
    let any_left = part.bound.iter().any(|shell| shell.version() >= SINCE);
    match (version >= SINCE, any_left) {
        (true, false) => {
            part.asks.clear();
            vec![Event::ShellGone]
        }
        (true, true) | (false, _) => Vec::new(),
    }
}
