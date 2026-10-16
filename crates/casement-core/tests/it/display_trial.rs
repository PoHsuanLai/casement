//! The display trial in the model: a display change by a program is applied, stored and put to
//! the person; Keep keeps it, Revert or the time running out puts the previous setup back; the
//! shell's own change is never asked.

use crate::support::driver::{Op, plug, run, run_from};
use crate::support::headless;
use casement_core::output::{DISPLAY_CONFIRM_WITHIN, OutputChangeSet, OutputConfig, Requester};
use casement_core::time::{Mono, TimerKey};
use casement_core::{Effect, Event, Model, OutputKey};
use casement_protocols::vocab::{TrialAnswer, TrialId};
use ds_core::geometry::scale::Scale;

fn key() -> OutputKey {
    headless("HEADLESS-1").key
}

/// A desktop with one output, and the configuration it starts with.
fn start() -> (Model, OutputConfig) {
    let plugged = run(vec![plug("HEADLESS-1")]);
    let original = plugged.model.outputs().0[&key()].config;
    (plugged.model, original)
}

/// The scale `steps` sixtieths of a unit above the original.
fn scaled(original: OutputConfig, steps: u32) -> Scale {
    Scale(original.scale.numerator() + 60 * steps)
}

fn rescale(by: Requester, scale: Scale) -> Op {
    let mut change = OutputChangeSet::none();
    change.scale = Some(scale);
    Op::Event(Event::OutputRequested {
        output: key(),
        change,
        by,
    })
}

fn answer(trial: TrialId, answer: TrialAnswer) -> Op {
    Op::Event(Event::DisplaysAnswered { trial, answer })
}

fn timeout() -> Op {
    Op::Event(Event::TimerFired(TimerKey::DisplayTrial))
}

fn config(model: &Model) -> OutputConfig {
    model.outputs().0[&key()].config
}

/// The trial the shell was asked about in `effects`.
fn asked(effects: &[Effect]) -> Option<TrialId> {
    effects.iter().find_map(|effect| match effect {
        Effect::AskKeepDisplays { trial, within } => {
            assert_eq!(*within, DISPLAY_CONFIRM_WITHIN);
            Some(*trial)
        }
        _ => None,
    })
}

fn ended(effects: &[Effect]) -> Vec<TrialId> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::EndDisplaysTrial { trial } => Some(*trial),
            _ => None,
        })
        .collect()
}

fn applied(effects: &[Effect]) -> Vec<OutputConfig> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::ApplyOutput { config, .. } => Some(*config),
            _ => None,
        })
        .collect()
}

fn armed(effects: &[Effect]) -> Vec<Mono> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::ArmTimer(due) if due.key == TimerKey::DisplayTrial => Some(due.at),
            _ => None,
        })
        .collect()
}

#[test]
fn a_programs_change_is_applied_stored_and_put_to_the_person() {
    let (model, original) = start();
    let want = scaled(original, 1);
    let step = run_from(model, vec![rescale(Requester::Program, want)]);
    let effects = &step.effects[0];
    assert_eq!(config(&step.model).scale, want, "applied at once");
    assert_eq!(applied(effects), [config(&step.model)]);
    assert!(asked(effects).is_some(), "the shell is asked: {effects:?}");
    assert_eq!(
        armed(effects),
        [step.model.now().after(DISPLAY_CONFIRM_WITHIN)]
    );
    assert!(step.model.timers().is_armed(&TimerKey::DisplayTrial));
}

#[test]
fn the_time_running_out_puts_the_previous_setup_back() {
    let (model, original) = start();
    let step = run_from(
        model,
        vec![rescale(Requester::Program, scaled(original, 1)), timeout()],
    );
    let trial = asked(&step.effects[0]).expect("asked");
    assert_eq!(config(&step.model), original);
    assert_eq!(ended(&step.effects[1]), [trial]);
    assert_eq!(applied(&step.effects[1]), [original]);
    assert!(!step.model.timers().is_armed(&TimerKey::DisplayTrial));
}

#[test]
fn keep_keeps_the_new_setup_and_a_late_timer_changes_nothing() {
    let (model, original) = start();
    let want = scaled(original, 1);
    let opened = run_from(model, vec![rescale(Requester::Program, want)]);
    let trial = asked(&opened.effects[0]).expect("asked");
    let kept = run_from(opened.model, vec![answer(trial, TrialAnswer::Keep)]);
    assert_eq!(config(&kept.model).scale, want);
    assert_eq!(ended(&kept.effects[0]), [trial]);
    assert!(applied(&kept.effects[0]).is_empty());
    assert!(!kept.model.timers().is_armed(&TimerKey::DisplayTrial));
    let late = run_from(kept.model, vec![timeout()]);
    assert_eq!(config(&late.model).scale, want, "nothing to revert");
    assert!(ended(&late.effects[0]).is_empty());
}

#[test]
fn revert_puts_the_previous_setup_back_at_once() {
    let (model, original) = start();
    let opened = run_from(
        model,
        vec![rescale(Requester::Program, scaled(original, 1))],
    );
    let trial = asked(&opened.effects[0]).expect("asked");
    let reverted = run_from(opened.model, vec![answer(trial, TrialAnswer::Revert)]);
    assert_eq!(config(&reverted.model), original);
    assert_eq!(applied(&reverted.effects[0]), [original]);
    assert_eq!(ended(&reverted.effects[0]), [trial]);
    assert!(!reverted.model.timers().is_armed(&TimerKey::DisplayTrial));
}

#[test]
fn the_shells_own_change_is_applied_and_never_asked() {
    let (model, original) = start();
    let want = scaled(original, 1);
    let step = run_from(model, vec![rescale(Requester::Shell, want)]);
    assert_eq!(config(&step.model).scale, want);
    assert!(asked(&step.effects[0]).is_none());
    assert!(!step.model.timers().is_armed(&TimerKey::DisplayTrial));
}

#[test]
fn the_shells_change_while_a_trial_is_open_keeps_it() {
    let (model, original) = start();
    let opened = run_from(
        model,
        vec![rescale(Requester::Program, scaled(original, 1))],
    );
    let trial = asked(&opened.effects[0]).expect("asked");
    let shell = scaled(original, 2);
    let step = run_from(
        opened.model,
        vec![rescale(Requester::Shell, shell), timeout()],
    );
    assert_eq!(ended(&step.effects[0]), [trial]);
    assert_eq!(
        config(&step.model).scale,
        shell,
        "the timer has nothing to revert"
    );
}

#[test]
fn the_compositors_own_restoring_leaves_a_trial_alone() {
    let (model, original) = start();
    let opened = run_from(
        model,
        vec![rescale(Requester::Program, scaled(original, 1))],
    );
    let step = run_from(
        opened.model,
        vec![rescale(Requester::Restore, scaled(original, 2))],
    );
    assert!(asked(&step.effects[0]).is_none());
    assert!(ended(&step.effects[0]).is_empty());
    assert!(step.model.timers().is_armed(&TimerKey::DisplayTrial));
}

#[test]
fn a_second_change_restarts_the_time_and_reverts_to_the_first_setup() {
    let (model, original) = start();
    let first = run_from(
        model,
        vec![rescale(Requester::Program, scaled(original, 1))],
    );
    let trial = asked(&first.effects[0]).expect("asked");
    let second = run_from(
        first.model,
        vec![rescale(Requester::Program, scaled(original, 2))],
    );
    assert_eq!(
        asked(&second.effects[0]),
        Some(trial),
        "the same question again"
    );
    assert_eq!(
        armed(&second.effects[0]),
        [second.model.now().after(DISPLAY_CONFIRM_WITHIN)],
        "the time restarts"
    );
    let reverted = run_from(second.model, vec![timeout()]);
    assert_eq!(
        config(&reverted.model),
        original,
        "not the first change's setup"
    );
    assert_eq!(ended(&reverted.effects[0]), [trial]);
}

#[test]
fn an_answer_for_a_trial_that_is_not_open_is_ignored() {
    let (model, original) = start();
    let want = scaled(original, 1);
    let opened = run_from(model, vec![rescale(Requester::Program, want)]);
    let trial = asked(&opened.effects[0]).expect("asked");
    let other = TrialId::new(trial.get() + 41);
    let step = run_from(opened.model, vec![answer(other, TrialAnswer::Revert)]);
    assert!(step.effects[0].is_empty(), "{:?}", step.effects[0]);
    assert_eq!(config(&step.model).scale, want);
    assert!(step.model.timers().is_armed(&TimerKey::DisplayTrial));
    let none = run(vec![plug("HEADLESS-1"), answer(other, TrialAnswer::Keep)]);
    assert!(none.effects[1].is_empty());
}
