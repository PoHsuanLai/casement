//! `quire_shell_v1` version 6, the display trial: a program changes the displays through
//! `zwlr_output_manager_v1`, the shell is asked whether to keep the change, and the previous
//! setup comes back on Revert or when the (virtual) time runs out. The shell's own changes are
//! never asked; a shell that binds mid-trial is asked with the time left; a shell older than
//! version 6 is never asked.

use super::consent::ScriptedShell;
use super::output_management::{boot, config_of, plan, settle};
use casement_core::output::DISPLAY_CONFIRM_WITHIN;
use casement_core::policy::ConnectionRole;
use casement_protocols::client::quire_shell_v1::quire_shell_v1 as wire;
use casement_protocols::vocab::TrialAnswer;
use casement_testclient::manage::output_management::{Answer, Intent, OutputManager, Properties};
use casement_testkit::Fixture;
use ds_core::geometry::scale::Scale;
use std::time::Duration;

const ONE: [&str; 1] = ["HEADLESS-1:1920x1080@60000:120:0,0"];

/// A fixture with one output, a shell bound at `version`, and a program's manager.
fn world(version: u32) -> (Fixture, ScriptedShell, OutputManager) {
    let (mut fixture, _program, manager) = boot(&ONE);
    let shell = ScriptedShell::start_at(&mut fixture, version);
    (fixture, shell, manager)
}

/// The program applies `scale` to the one output; the compositor takes it.
fn rescale(fixture: &mut Fixture, manager: &mut OutputManager, scale: f64) {
    let properties = Properties {
        scale: Some(scale),
        ..Properties::default()
    };
    let wanted = manager
        .configure(None, &[plan("HEADLESS-1", properties)], Intent::Apply)
        .expect("a configuration");
    settle(fixture, manager);
    assert_eq!(manager.answer(wanted), Some(Answer::Succeeded));
}

fn scale_of(fixture: &Fixture) -> Scale {
    config_of(fixture, "HEADLESS-1").scale
}

/// The one trial the shell heard.
fn only_trial(shell: &ScriptedShell) -> u32 {
    let trials = shell.trials();
    assert_eq!(trials.len(), 1, "one trial: {trials:?}");
    trials[0].0
}

#[test]
fn a_change_nobody_keeps_is_reverted_when_the_time_runs_out() {
    let (mut fixture, shell, mut manager) = world(6);
    let start = scale_of(&fixture);
    rescale(&mut fixture, &mut manager, 2.0);
    assert_eq!(scale_of(&fixture), Scale(240));
    fixture.pump();
    let trial = only_trial(&shell);
    assert_eq!(
        shell.trials()[0].1,
        u32::try_from(DISPLAY_CONFIRM_WITHIN.as_secs()).expect("seconds")
    );
    fixture.advance(DISPLAY_CONFIRM_WITHIN - Duration::from_secs(1));
    assert_eq!(scale_of(&fixture), Scale(240), "still on trial");
    fixture.advance(Duration::from_secs(2));
    fixture.pump();
    assert_eq!(scale_of(&fixture), start);
    assert_eq!(shell.trial_ends(), [trial]);
}

#[test]
fn keep_keeps_the_change_for_good() {
    let (mut fixture, shell, mut manager) = world(6);
    rescale(&mut fixture, &mut manager, 2.0);
    fixture.pump();
    let trial = only_trial(&shell);
    shell.answer_trial(&mut fixture, trial, TrialAnswer::Keep);
    assert_eq!(shell.trial_ends(), [trial]);
    fixture.advance(DISPLAY_CONFIRM_WITHIN * 2);
    assert_eq!(scale_of(&fixture), Scale(240));
}

#[test]
fn revert_puts_the_previous_setup_back_at_once() {
    let (mut fixture, shell, mut manager) = world(6);
    let start = scale_of(&fixture);
    rescale(&mut fixture, &mut manager, 2.0);
    fixture.pump();
    let trial = only_trial(&shell);
    shell.answer_trial(&mut fixture, trial, TrialAnswer::Revert);
    assert_eq!(scale_of(&fixture), start);
    assert_eq!(shell.trial_ends(), [trial]);
}

#[test]
fn the_shells_own_change_is_applied_and_not_asked() {
    let (mut fixture, shell, _program) = world(6);
    let own = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    let mut theirs =
        OutputManager::start(fixture.client(own).globals()).expect("the shell's manager");
    settle(&mut fixture, &mut theirs);
    rescale(&mut fixture, &mut theirs, 2.0);
    fixture.advance(DISPLAY_CONFIRM_WITHIN * 2);
    fixture.pump();
    assert_eq!(scale_of(&fixture), Scale(240), "nothing came back");
    assert!(shell.trials().is_empty(), "{:?}", shell.trials());
}

#[test]
fn a_second_change_restarts_the_time() {
    let (mut fixture, shell, mut manager) = world(6);
    let start = scale_of(&fixture);
    rescale(&mut fixture, &mut manager, 1.5);
    fixture.advance(Duration::from_secs(10));
    rescale(&mut fixture, &mut manager, 2.0);
    fixture.pump();
    let trials = shell.trials();
    assert_eq!(trials.len(), 2, "asked again: {trials:?}");
    assert_eq!(trials[0].0, trials[1].0, "the same question");
    assert_eq!(u64::from(trials[1].1), DISPLAY_CONFIRM_WITHIN.as_secs());
    // Ten seconds after the second change is past the first change's fifteen.
    fixture.advance(Duration::from_secs(10));
    assert_eq!(scale_of(&fixture), Scale(240), "the time restarted");
    fixture.advance(Duration::from_secs(6));
    assert_eq!(scale_of(&fixture), start, "back to the setup before both");
}

#[test]
fn a_shell_that_binds_mid_trial_is_asked_with_the_time_left() {
    let (mut fixture, _shell, mut manager) = world(6);
    rescale(&mut fixture, &mut manager, 2.0);
    fixture.advance(Duration::from_secs(5));
    let late = ScriptedShell::start(&mut fixture);
    let trials = late.trials();
    assert_eq!(trials.len(), 1, "{trials:?}");
    assert_eq!(u64::from(trials[0].1), DISPLAY_CONFIRM_WITHIN.as_secs() - 5);
}

#[test]
fn an_answer_that_comes_too_soon_is_the_protocol_error_and_the_trial_stays() {
    let (mut fixture, shell, mut manager) = world(6);
    rescale(&mut fixture, &mut manager, 2.0);
    fixture.pump();
    let trial = only_trial(&shell);
    fixture.advance(Duration::from_millis(100));
    shell.answer_trial_at_once(&mut fixture, trial, TrialAnswer::Keep);
    let error = fixture.client(shell.id).protocol_error().expect("too soon");
    assert_eq!(error.interface, "quire_shell_v1");
    assert_eq!(error.code, wire::Error::AnsweredTooSoon as u32);
    assert!(shell.trial_ends().is_empty(), "the trial is still open");
}

#[test]
fn an_answer_for_a_trial_that_is_not_open_is_ignored() {
    let (mut fixture, shell, mut manager) = world(6);
    shell.answer_trial(&mut fixture, 77, TrialAnswer::Revert);
    assert!(fixture.client(shell.id).protocol_error().is_none());
    rescale(&mut fixture, &mut manager, 2.0);
    fixture.pump();
    let trial = only_trial(&shell);
    shell.answer_trial(&mut fixture, trial + 1, TrialAnswer::Revert);
    assert!(fixture.client(shell.id).protocol_error().is_none());
    assert_eq!(
        scale_of(&fixture),
        Scale(240),
        "another trial's answer changes nothing"
    );
    assert!(shell.trial_ends().is_empty());
}

#[test]
fn a_shell_older_than_version_six_is_not_asked_and_the_time_reverts_the_change() {
    let (mut fixture, shell, mut manager) = world(5);
    let start = scale_of(&fixture);
    rescale(&mut fixture, &mut manager, 2.0);
    fixture.pump();
    assert!(shell.trials().is_empty());
    fixture.advance(DISPLAY_CONFIRM_WITHIN + Duration::from_secs(1));
    assert_eq!(scale_of(&fixture), start);
    assert!(shell.trial_ends().is_empty(), "it never heard of the trial");
}

#[test]
fn a_change_with_no_shell_at_all_is_reverted_when_the_time_runs_out() {
    let (mut fixture, _program, mut manager) = boot(&ONE);
    let start = scale_of(&fixture);
    rescale(&mut fixture, &mut manager, 2.0);
    fixture.advance(DISPLAY_CONFIRM_WITHIN + Duration::from_secs(1));
    assert_eq!(scale_of(&fixture), start);
}
