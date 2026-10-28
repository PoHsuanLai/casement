//! `gamma` (P1-M1): the colour curve of an output. A ramp a client sets is recorded by the
//! headless backend, so the test reads it back.

use super::consent::{ScriptedShell, kept, program};
use casement_core::output::{GammaRamp, GammaTable};
use casement_core::policy::ConnectionRole;
use casement_core::privilege::{HELD_REQUESTS, UnixSeconds, Verdict};
use casement_core::{Event, OutputName};
use casement_protocols::vocab::{Answer, Power};
use casement_testclient::manage::gamma::{ControlId, ControlState, GammaControls};
use casement_testkit::{ASKED_PROGRAM, ClientId, Fixture, FixtureSpec};

/// A fixture with one output, a client and a gamma helper that has read the output name.
fn boot() -> (Fixture, ClientId, GammaControls) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut gamma = GammaControls::start(fixture.client(id).globals()).expect("the manager");
    settle(&mut fixture, &mut gamma);
    (fixture, id, gamma)
}

/// Lets the compositor and the helper talk until both are quiet.
fn settle(fixture: &mut Fixture, gamma: &mut GammaControls) {
    fixture.pump();
    gamma.pump().expect("pump");
    fixture.pump();
    gamma.pump().expect("pump");
}

fn output() -> OutputName {
    OutputName::parse("HEADLESS-1").expect("a connector name")
}

fn steps(size: u32) -> (Vec<u16>, Vec<u16>, Vec<u16>) {
    let up: Vec<u16> = (0..size)
        .map(|n| u16::try_from(n * 256).unwrap_or(u16::MAX))
        .collect();
    let down: Vec<u16> = up.iter().rev().copied().collect();
    (up.clone(), vec![0x8000; up.len()], down)
}

/// The control announces the table size; a ramp of that size is accepted and reaches the
/// backend as `Effect::SetGamma`.
#[test]
fn a_ramp_of_the_announced_size_reaches_the_backend() {
    let (mut fixture, id, mut gamma) = boot();
    assert_eq!(fixture.gamma_of(&output()), Some(GammaRamp::Identity));
    let control = gamma.control("HEADLESS-1").expect("a control");
    settle(&mut fixture, &mut gamma);
    let Some(ControlState::Sized(size)) = gamma.state(control) else {
        panic!("the control announces its size: {:?}", gamma.state(control));
    };
    assert!(size > 0);

    let (red, green, blue) = steps(size);
    gamma
        .set_ramp(control, &red, &green, &blue)
        .expect("a ramp");
    settle(&mut fixture, &mut gamma);
    // The ramp is raised to the floor a client may not go under (CASW-18).
    let table = GammaTable::new(red, green, blue)
        .expect("a table")
        .floored();
    assert_eq!(fixture.gamma_of(&output()), Some(GammaRamp::Table(table)));
    assert!(fixture.client(id).protocol_error().is_none());
}

/// Destroying the control restores the identity curve; a second control on a controlled output
/// is `failed`.
#[test]
fn destroying_restores_identity_and_a_second_control_fails() {
    let (mut fixture, _, mut gamma) = boot();
    let control = gamma.control("HEADLESS-1").expect("a control");
    let second = gamma.control("HEADLESS-1").expect("a second control");
    settle(&mut fixture, &mut gamma);
    let Some(ControlState::Sized(size)) = gamma.state(control) else {
        panic!("the control announces its size");
    };
    assert_eq!(gamma.state(second), Some(ControlState::Failed));

    let (red, green, blue) = steps(size);
    gamma
        .set_ramp(control, &red, &green, &blue)
        .expect("a ramp");
    settle(&mut fixture, &mut gamma);
    assert!(matches!(
        fixture.gamma_of(&output()),
        Some(GammaRamp::Table(_))
    ));

    gamma.destroy(control).expect("destroyed");
    settle(&mut fixture, &mut gamma);
    assert_eq!(fixture.gamma_of(&output()), Some(GammaRamp::Identity));

    let again = gamma
        .control("HEADLESS-1")
        .expect("a control once it is free");
    settle(&mut fixture, &mut gamma);
    assert_eq!(gamma.state(again), Some(ControlState::Sized(size)));
}

/// A ramp of the wrong size is `invalid_gamma`, and nothing reaches the backend.
#[test]
fn a_wrong_size_is_an_error() {
    let (mut fixture, id, mut gamma) = boot();
    let control = gamma.control("HEADLESS-1").expect("a control");
    settle(&mut fixture, &mut gamma);
    let Some(ControlState::Sized(size)) = gamma.state(control) else {
        panic!("the control announces its size");
    };
    let (red, green, blue) = steps(size - 1);
    gamma
        .set_ramp(control, &red, &green, &blue)
        .expect("a ramp");
    fixture.pump();
    let error = fixture.client(id).protocol_error().expect("an error");
    assert_eq!(error.interface, "zwlr_gamma_control_v1");
    assert_eq!(error.code, 1);
    assert_eq!(fixture.gamma_of(&output()), Some(GammaRamp::Identity));
}

/// CASW-18: a client that asks for black gets a dim screen, not a black one, and its going
/// restores the identity curve.
#[test]
fn an_all_zero_ramp_is_raised_to_the_floor() {
    let (mut fixture, _, mut gamma) = boot();
    let control = gamma.control("HEADLESS-1").expect("a control");
    settle(&mut fixture, &mut gamma);
    let Some(ControlState::Sized(size)) = gamma.state(control) else {
        panic!("the control announces its size");
    };
    let zeros = vec![0u16; usize::try_from(size).expect("a size")];
    gamma
        .set_ramp(control, &zeros, &zeros, &zeros)
        .expect("a ramp");
    settle(&mut fixture, &mut gamma);
    let Some(GammaRamp::Table(table)) = fixture.gamma_of(&output()) else {
        panic!("a table reached the backend");
    };
    assert!(
        table.red().last().is_some_and(|top| *top >= 8_000),
        "the brightest step is lifted: {:?}",
        table.red().last()
    );
    assert_eq!(table.red(), table.green());
    gamma.destroy(control).expect("destroyed");
    settle(&mut fixture, &mut gamma);
    assert_eq!(fixture.gamma_of(&output()), Some(GammaRamp::Identity));
}

/// A fixture with one output, the scripted shell, and a gamma helper held by a program that is
/// asked about its powers.
fn asked_boot() -> (Fixture, ScriptedShell, GammaControls) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let shell = ScriptedShell::start(&mut fixture);
    let id = fixture.connect_via_ask();
    fixture.pump();
    let mut gamma = GammaControls::start(fixture.client(id).globals()).expect("the manager");
    settle(&mut fixture, &mut gamma);
    (fixture, shell, gamma)
}

/// A ramp of the size the headless backend drives, which a waiting control was never told.
fn headless_ramp() -> (Vec<u16>, Vec<u16>, Vec<u16>) {
    steps(u32::try_from(casement_backend::headless::GAMMA_SIZE).expect("a size"))
}

/// A control made while the question about the displays is open is inert: it hears no table
/// size and a ramp it sends changes nothing. When the person allows it is told the size, and
/// works.
#[test]
fn a_control_made_while_asked_is_inert_until_the_person_allows() {
    let (mut fixture, shell, mut gamma) = asked_boot();
    let control = gamma.control("HEADLESS-1").expect("a control");
    settle(&mut fixture, &mut gamma);
    assert_eq!(shell.asked().len(), 1, "the shell is asked once");
    assert_eq!(shell.asked()[0].power, Power::Displays);
    assert_eq!(gamma.state(control), Some(ControlState::Silent));

    let (red, green, blue) = headless_ramp();
    gamma
        .set_ramp(control, &red, &green, &blue)
        .expect("a ramp");
    settle(&mut fixture, &mut gamma);
    assert_eq!(fixture.gamma_of(&output()), Some(GammaRamp::Identity));

    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    settle(&mut fixture, &mut gamma);
    let Some(ControlState::Sized(size)) = gamma.state(control) else {
        panic!("the control is told its size: {:?}", gamma.state(control));
    };
    let (red, green, blue) = steps(size);
    gamma
        .set_ramp(control, &red, &green, &blue)
        .expect("a ramp");
    settle(&mut fixture, &mut gamma);
    assert!(matches!(
        fixture.gamma_of(&output()),
        Some(GammaRamp::Table(_))
    ));
}

/// Don't Allow fails the waiting control, and a ramp it sends changes nothing.
#[test]
fn a_control_made_while_asked_fails_when_the_person_refuses() {
    let (mut fixture, shell, mut gamma) = asked_boot();
    let control = gamma.control("HEADLESS-1").expect("a control");
    settle(&mut fixture, &mut gamma);
    shell.answer(&mut fixture, shell.only_ask(), Answer::Deny);
    settle(&mut fixture, &mut gamma);
    assert_eq!(gamma.state(control), Some(ControlState::Failed));
    let (red, green, blue) = headless_ramp();
    gamma
        .set_ramp(control, &red, &green, &blue)
        .expect("a ramp");
    settle(&mut fixture, &mut gamma);
    assert_eq!(fixture.gamma_of(&output()), Some(GammaRamp::Identity));
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Displays),
        Some(Verdict::Deny)
    );
}

/// Taking the power back in Settings fails the control in use at once and restores the identity
/// curve it had changed.
#[test]
fn a_power_taken_back_fails_the_control_and_restores_the_curve() {
    let (mut fixture, shell, mut gamma) = asked_boot();
    let control = gamma.control("HEADLESS-1").expect("a control");
    settle(&mut fixture, &mut gamma);
    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    settle(&mut fixture, &mut gamma);
    let Some(ControlState::Sized(size)) = gamma.state(control) else {
        panic!("the control is told its size");
    };
    let (red, green, blue) = steps(size);
    gamma
        .set_ramp(control, &red, &green, &blue)
        .expect("a ramp");
    settle(&mut fixture, &mut gamma);
    assert!(matches!(
        fixture.gamma_of(&output()),
        Some(GammaRamp::Table(_))
    ));

    fixture.feed_events(vec![Event::PrivilegeEdited {
        edit: casement_core::privilege::Edit::Deny(program(ASKED_PROGRAM), Power::Displays),
        at: UnixSeconds(2),
    }]);
    settle(&mut fixture, &mut gamma);
    assert_eq!(gamma.state(control), Some(ControlState::Failed));
    assert_eq!(fixture.gamma_of(&output()), Some(GammaRamp::Identity));
    gamma
        .set_ramp(control, &red, &green, &blue)
        .expect("a ramp");
    settle(&mut fixture, &mut gamma);
    assert_eq!(fixture.gamma_of(&output()), Some(GammaRamp::Identity));
}

/// A client keeps at most `HELD_REQUESTS` controls waiting for the answer; the next one fails
/// at once.
#[test]
fn a_client_holds_at_most_the_cap_of_waiting_controls() {
    let (mut fixture, _shell, mut gamma) = asked_boot();
    let controls: Vec<ControlId> = (0..=HELD_REQUESTS)
        .map(|_| gamma.control("HEADLESS-1").expect("a control"))
        .collect();
    settle(&mut fixture, &mut gamma);
    let failed: Vec<ControlId> = controls
        .iter()
        .copied()
        .filter(|control| gamma.state(*control) == Some(ControlState::Failed))
        .collect();
    assert_eq!(
        failed,
        [controls[HELD_REQUESTS]],
        "only the one past the cap"
    );
}
