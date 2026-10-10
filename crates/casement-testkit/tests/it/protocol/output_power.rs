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

/// Turning an output off darkens it (no more frames presented) and the control reports `off`;
/// turning it on lights it again.
#[test]
fn power_off_darkens_a_virtual_output_and_on_lights_it() {
    let (mut fixture, id, mut powers) = boot(&TWO[..1]);
    let control = powers.control("HEADLESS-1").expect("a control");
    settle(&mut fixture, &mut powers);
    assert_eq!(
        powers.state(control),
        Some(ControlState::Reported(PowerMode::On)),
        "a new control says what the power is"
    );
    assert!(fixture.advance(Duration::from_millis(100)).frames > 0);

    powers.set_mode(control, PowerMode::Off).expect("off");
    settle(&mut fixture, &mut powers);
    assert_eq!(power_of(&fixture, "HEADLESS-1"), OutputPower::Off);
    assert_eq!(powers.reported(control), [PowerMode::On, PowerMode::Off]);
    assert_eq!(fixture.advance(Duration::from_millis(100)).frames, 0);

    powers.set_mode(control, PowerMode::On).expect("on");
    settle(&mut fixture, &mut powers);
    assert_eq!(power_of(&fixture, "HEADLESS-1"), OutputPower::On);
    assert_eq!(
        powers.reported(control),
        [PowerMode::On, PowerMode::Off, PowerMode::On]
    );
    assert!(fixture.advance(Duration::from_millis(100)).frames > 0);
    assert!(fixture.client(id).protocol_error().is_none());
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
