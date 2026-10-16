//! `output_power` (P1-M1): output power works on virtual outputs.

use casement_core::OutputName;
use casement_core::output::OutputPower;
use casement_core::policy::ConnectionRole;
use casement_testclient::manage::output_power::{ControlState, OutputPowers, PowerMode};
use casement_testkit::{ClientId, Fixture, FixtureSpec};
use std::time::Duration;

const TWO: [&str; 2] = [
    "HEADLESS-1:1920x1080@60000:120:0,0",
    "HEADLESS-2:1920x1080@60000:120:1920,0",
];

/// A fixture with `outputs`, a client and a power manager that has read the output names.
fn boot(outputs: &[&str]) -> (Fixture, ClientId, OutputPowers) {
    let mut fixture =
        Fixture::start(FixtureSpec::desktop().with_outputs(outputs)).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut powers = OutputPowers::start(fixture.client(id).globals()).expect("the manager");
    settle(&mut fixture, &mut powers);
    (fixture, id, powers)
}

/// Lets the compositor and the helper talk until both are quiet.
fn settle(fixture: &mut Fixture, powers: &mut OutputPowers) {
    fixture.pump();
    powers.pump().expect("pump");
    fixture.pump();
    powers.pump().expect("pump");
}

fn power_of(fixture: &Fixture, connector: &str) -> OutputPower {
    let wanted = OutputName::parse(connector).expect("a connector name");
    fixture
        .model()
        .outputs()
        .0
        .values()
        .find(|state| state.facts.name == wanted)
        .map(|state| state.config.power)
        .expect("a plugged output")
}

/// Turning an output off darkens it (it presents no more frames, the other one still does) and
/// the control reports `off`; turning it on lights it again.
#[test]
fn power_off_darkens_a_virtual_output_and_on_lights_it() {
    let (mut fixture, id, mut powers) = boot(&TWO);
    let control = powers.control("HEADLESS-1").expect("a control");
    settle(&mut fixture, &mut powers);
    assert_eq!(
        powers.state(control),
        Some(ControlState::Reported(PowerMode::On)),
        "a new control says what the power is"
    );
    let both = fixture.advance(Duration::from_millis(100)).frames;
    assert!(both > 0);

    powers.set_mode(control, PowerMode::Off).expect("off");
    settle(&mut fixture, &mut powers);
    assert_eq!(power_of(&fixture, "HEADLESS-1"), OutputPower::Off);
    assert_eq!(powers.reported(control), [PowerMode::On, PowerMode::Off]);
    let one = fixture.advance(Duration::from_millis(100)).frames;
    assert!(
        one > 0 && one < both,
        "only the lit output presents: {one} of {both}"
    );

    powers.set_mode(control, PowerMode::On).expect("on");
    settle(&mut fixture, &mut powers);
    assert_eq!(power_of(&fixture, "HEADLESS-1"), OutputPower::On);
    assert_eq!(
        powers.reported(control),
        [PowerMode::On, PowerMode::Off, PowerMode::On]
    );
    assert!(fixture.advance(Duration::from_millis(100)).frames > one);
    assert!(fixture.client(id).protocol_error().is_none());
}

/// CASC-09: the last lit output cannot be darkened by a client.
#[test]
fn the_last_lit_output_stays_lit() {
    let (mut fixture, _, mut powers) = boot(&TWO);
    let one = powers.control("HEADLESS-1").expect("a control");
    let two = powers.control("HEADLESS-2").expect("a control");
    settle(&mut fixture, &mut powers);
    powers.set_mode(one, PowerMode::Off).expect("off");
    settle(&mut fixture, &mut powers);
    powers.set_mode(two, PowerMode::Off).expect("off");
    settle(&mut fixture, &mut powers);
    assert_eq!(power_of(&fixture, "HEADLESS-1"), OutputPower::Off);
    assert_eq!(power_of(&fixture, "HEADLESS-2"), OutputPower::On, "refused");
    assert!(fixture.advance(Duration::from_millis(100)).frames > 0);
}

/// CASC-09: an output a client darkened is lit again when its control goes, or its client does.
#[test]
fn a_darkened_output_is_lit_again_when_the_control_goes() {
    let (mut fixture, id, mut powers) = boot(&TWO);
    let control = powers.control("HEADLESS-1").expect("a control");
    settle(&mut fixture, &mut powers);
    powers.set_mode(control, PowerMode::Off).expect("off");
    settle(&mut fixture, &mut powers);
    assert_eq!(power_of(&fixture, "HEADLESS-1"), OutputPower::Off);
    powers.destroy(control).expect("destroyed");
    settle(&mut fixture, &mut powers);
    assert_eq!(power_of(&fixture, "HEADLESS-1"), OutputPower::On);

    // The same when the client goes without destroying anything.
    let again = powers.control("HEADLESS-2").expect("a control");
    settle(&mut fixture, &mut powers);
    powers.set_mode(again, PowerMode::Off).expect("off");
    settle(&mut fixture, &mut powers);
    assert_eq!(power_of(&fixture, "HEADLESS-2"), OutputPower::Off);
    drop(powers);
    fixture.disconnect(id);
    assert_eq!(power_of(&fixture, "HEADLESS-2"), OutputPower::On);
}

/// A second control on the same output is `failed`; once the first is destroyed a new one works.
#[test]
fn a_second_control_on_an_output_fails() {
    let (mut fixture, _, mut powers) = boot(&TWO[..1]);
    let first = powers.control("HEADLESS-1").expect("a control");
    let second = powers.control("HEADLESS-1").expect("a second control");
    settle(&mut fixture, &mut powers);
    assert_eq!(
        powers.state(first),
        Some(ControlState::Reported(PowerMode::On))
    );
    assert_eq!(powers.state(second), Some(ControlState::Failed));

    powers.destroy(first).expect("destroyed");
    settle(&mut fixture, &mut powers);
    let third = powers.control("HEADLESS-1").expect("a third control");
    settle(&mut fixture, &mut powers);
    assert_eq!(
        powers.state(third),
        Some(ControlState::Reported(PowerMode::On))
    );
}

/// Controls on different outputs are independent, and a control on an unplugged output fails.
#[test]
fn controls_follow_their_own_output() {
    let (mut fixture, _, mut powers) = boot(&TWO);
    let one = powers.control("HEADLESS-1").expect("a control");
    let two = powers.control("HEADLESS-2").expect("a control");
    settle(&mut fixture, &mut powers);
    powers.set_mode(two, PowerMode::Off).expect("off");
    settle(&mut fixture, &mut powers);
    assert_eq!(power_of(&fixture, "HEADLESS-1"), OutputPower::On);
    assert_eq!(power_of(&fixture, "HEADLESS-2"), OutputPower::Off);
    assert_eq!(powers.reported(one), [PowerMode::On]);

    fixture.unplug(&OutputName::parse("HEADLESS-2").expect("a name"));
    settle(&mut fixture, &mut powers);
    assert_eq!(powers.state(two), Some(ControlState::Failed));
    assert_eq!(
        powers.state(one),
        Some(ControlState::Reported(PowerMode::On))
    );
}
