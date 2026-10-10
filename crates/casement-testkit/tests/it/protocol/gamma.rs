//! `gamma` (P1-M1): the colour curve of an output. A ramp a client sets is recorded by the
//! headless backend, so the test reads it back.

use casement_core::OutputName;
use casement_core::output::{GammaRamp, GammaTable};
use casement_core::policy::ConnectionRole;
use casement_testclient::manage::gamma::{ControlState, GammaControls};
use casement_testkit::{ClientId, Fixture, FixtureSpec};

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
    let table = GammaTable::new(red, green, blue).expect("a table");
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
