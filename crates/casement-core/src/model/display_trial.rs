//! The display trial: a change to the displays by a program is applied at once and the person
//! decides whether it stays.
//!
//! A change from a [`Requester::Program`] stores the output's previous configuration, arms
//! `TimerKey::DisplayTrial` for [`DISPLAY_CONFIRM_WITHIN`] and asks the shell
//! ([`Effect::AskKeepDisplays`]). Keep keeps it. Revert, or the time running out, applies the
//! stored configurations again. A further change while the trial is open keeps the first
//! stored configuration of each output, restarts the time and asks again with the same id. A
//! change by the shell is the person's own: it ends a trial in flight as kept. The compositor's
//! own restoring ([`Requester::Restore`]) and hotplug never start a trial.
//!
//! The trial is open whether or not a shell can carry the question: one that cannot (it bound
//! before the question existed, or none is bound) never hears it, and the time running out
//! is what puts the previous setup back.

use super::Model;
use super::moved::Moved;
use crate::effect::Effect;
use crate::ids::OutputKey;
use crate::output::{DISPLAY_CONFIRM_WITHIN, OutputConfig, Requester};
use crate::time::{Due, TimerKey};
use casement_protocols::vocab::{TrialAnswer, TrialId};
use std::collections::BTreeMap;

/// A display change by a program that waits for the person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Trial {
    /// The id the question carries on the wire.
    id: TrialId,
    /// Every output the trial changed, as it was before the trial's first change to it.
    before: BTreeMap<OutputKey, OutputConfig>,
}

impl Model {
    /// `output` was changed by `by` and was `before` until now.
    pub(super) fn display_changed(
        self,
        by: Requester,
        output: OutputKey,
        before: OutputConfig,
    ) -> Moved {
        match by {
            Requester::Program => self.trial_asked(output, before),
            Requester::Shell => self.trial_over(false),
            Requester::Restore => Moved::of(self),
        }
    }

    /// The person answered the question about trial `trial`. An answer for a trial that is not
    /// the open one changes nothing.
    pub(super) fn display_trial_answered(self, trial: TrialId, answer: TrialAnswer) -> Moved {
        let ours = self
            .display_trial
            .as_ref()
            .is_some_and(|open| open.id == trial);
        match (ours, answer) {
            (true, TrialAnswer::Keep) => self.trial_over(false),
            (true, TrialAnswer::Revert) => self.trial_over(true),
            (false, _) => Moved::of(self),
        }
    }

    /// The person did not answer in time: the previous setup comes back.
    pub(super) fn display_trial_timed_out(self) -> Moved {
        self.trial_over(true)
    }

    /// A trial opens, or the open one is restarted by a further change; the shell is asked.
    fn trial_asked(self, output: OutputKey, before: OutputConfig) -> Moved {
        let mut model = self;
        let (id, mut stored) = match model.display_trial.take() {
            Some(open) => (open.id, open.before),
            None => (model.next_trial_id(), BTreeMap::new()),
        };
        stored.entry(output).or_insert(before);
        model.display_trial = Some(Trial { id, before: stored });
        let due = Due {
            at: model.now.after(DISPLAY_CONFIRM_WITHIN),
            key: TimerKey::DisplayTrial,
        };
        model.timers = model.timers.arm(due.clone());
        Moved::of(model)
            .with(Effect::ArmTimer(due))
            .with(Effect::AskKeepDisplays {
                trial: id,
                within: DISPLAY_CONFIRM_WITHIN,
            })
    }

    /// The open trial, if any, ends: the shell is told, the timer goes, and on a `revert` every
    /// output it changed gets its previous configuration back (otherwise the new setup stays).
    fn trial_over(self, revert: bool) -> Moved {
        let mut model = self;
        let Some(trial) = model.display_trial.take() else {
            return Moved::of(model);
        };
        let armed = model.timers.is_armed(&TimerKey::DisplayTrial);
        model.timers = model.timers.cancel(&TimerKey::DisplayTrial);
        let ended = Moved::of(model).with(Effect::EndDisplaysTrial { trial: trial.id });
        let ended = match armed {
            true => ended.with(Effect::CancelTimer(TimerKey::DisplayTrial)),
            false => ended,
        };
        match revert {
            false => ended,
            true => trial
                .before
                .into_iter()
                .fold(ended, |moved, (output, config)| {
                    moved.then(|model| model.put_back(&output, config))
                }),
        }
    }

    /// `output` is configured `config` again, when it is still plugged and is not already.
    fn put_back(self, output: &OutputKey, config: OutputConfig) -> Moved {
        let differs = self
            .outputs
            .0
            .get(output)
            .is_some_and(|plugged| plugged.config != config);
        match differs {
            true => Moved::of(self)
                .with(Effect::ApplyOutput {
                    output: output.clone(),
                    config,
                })
                .then(|model| model.configure_output(output, config)),
            false => Moved::of(self),
        }
    }

    /// The id of the next trial; never the same twice in a run.
    fn next_trial_id(&mut self) -> TrialId {
        self.trial_ids = self.trial_ids.wrapping_add(1);
        TrialId::new(self.trial_ids)
    }
}
