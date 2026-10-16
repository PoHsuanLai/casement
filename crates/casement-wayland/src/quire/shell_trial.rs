//! `quire_shell_v1` version 6: the display trial. A program changed the displays; the compositor
//! applied the change and asks the person, through the shell, whether to keep it
//! (`displays_trial`), takes the question down (`displays_trial_end`) and hears the answer
//! (`displays_trial_answer`). The model decides when to ask and what an answer does
//! (`casement_core`'s `model/display_trial.rs`); this only carries the words.
//!
//! The part remembers the one open trial, so a shell that binds while it is open (it
//! restarted) is asked at once with the time left. A shell that bound a version older than 6
//! hears nothing: the change is reverted when the time runs out. An answer comes after the same
//! margin as a permission answer ([`ANSWER_NOT_BEFORE`]); one for a trial that is not open is
//! ignored.

use super::code_of;
use super::shell::QuireShellPart;
use super::shell_permission::{ANSWER_NOT_BEFORE, AnsweredTooSoon};
use casement_core::Event;
use casement_core::time::Mono;
use casement_protocols::server::quire_shell_v1::quire_shell_v1::{self, QuireShellV1};
use casement_protocols::vocab::{TrialAnswer, TrialId, WireCode};
use smithay::reexports::wayland_server::{Resource, WEnum};
use std::time::Duration;

/// The first version of `quire_shell_v1` that carries the trial.
pub const SINCE: u32 = 6;

/// The trial the shell was asked about and has not seen answered or ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenTrial {
    id: TrialId,
    /// When the question was first sent (the loop's clock). A further change restarts the
    /// time and sends the question again but does not move this: the sheet was already on
    /// screen, and a person who pressed a key on it just before it restarted is not at fault.
    sent_at: Mono,
    /// When the time runs out and the change is reverted.
    ends: Mono,
}

/// Whole seconds, rounded up: a countdown never shows less than the person has.
fn whole_seconds(left: Duration) -> u32 {
    let rounded = left.as_secs() + u64::from(left.subsec_nanos() > 0);
    u32::try_from(rounded).unwrap_or(u32::MAX)
}

/// Sends `displays_trial` to `shell` when the object is new enough to carry it.
fn send(shell: &QuireShellV1, trial: TrialId, left: Duration) {
    if shell.version() >= SINCE {
        shell.displays_trial(trial.get(), whole_seconds(left));
    }
}

/// `Effect::AskKeepDisplays`: puts the question to every bound shell object that can carry it,
/// at `now` on the loop's clock, with `within` to answer.
pub fn apply_trial(part: &mut QuireShellPart, trial: TrialId, within: Duration, now: Mono) {
    let sent_at = match part.trial {
        Some(open) if open.id == trial => open.sent_at,
        Some(_) | None => now,
    };
    part.trial = Some(OpenTrial {
        id: trial,
        sent_at,
        ends: now.after(within),
    });
    part.bound
        .iter()
        .for_each(|shell| send(shell, trial, within));
}

/// `Effect::EndDisplaysTrial`: takes the question down from every bound shell object.
pub fn apply_end(part: &mut QuireShellPart, trial: TrialId) {
    if part.trial.is_some_and(|open| open.id == trial) {
        part.trial = None;
    }
    for shell in &part.bound {
        if shell.version() >= SINCE {
            shell.displays_trial_end(trial.get());
        }
    }
}

/// `displays_trial_answer`: the person's word on `trial`, heard at `now`. An answer for a trial
/// that is not open, and one with a code the XML does not name, changes nothing; one that comes
/// less than [`ANSWER_NOT_BEFORE`] after the question was first sent is the shell's fault
/// (`AnsweredTooSoon`) and settles nothing: the trial stays open.
pub fn on_answer(
    part: &QuireShellPart,
    trial: u32,
    answer: WEnum<quire_shell_v1::TrialAnswer>,
    now: Mono,
) -> Result<Vec<Event>, AnsweredTooSoon> {
    let Some(open) = part.trial.filter(|open| open.id.get() == trial) else {
        return Ok(Vec::new());
    };
    if now.since(open.sent_at) < ANSWER_NOT_BEFORE {
        return Err(AnsweredTooSoon);
    }
    let Some(answer) = code_of(answer).and_then(|code| TrialAnswer::from_code(code).ok()) else {
        return Ok(Vec::new());
    };
    Ok(vec![Event::DisplaysAnswered {
        trial: open.id,
        answer,
    }])
}

/// A shell object bound: when a trial is open it is asked at once with the time left.
pub fn on_bind(part: &QuireShellPart, shell: &QuireShellV1, now: Mono) {
    if let Some(open) = part.trial {
        let left = open.ends.since(now);
        if !left.is_zero() {
            send(shell, open.id, left);
        }
    }
}
