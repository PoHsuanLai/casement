//! A client's output change request, resolved against the display's facts (P1-M1). Ignored
//! until the lane fills `resolve_change`.

use crate::support::{display, mode};
use casement_core::geometry::{LogicalPoint, Transform};
use casement_core::output::{
    ModePreference, OutputChangeSet, OutputConfig, OutputPower, OutputRefusal, resolve_change,
};
use ds_core::geometry::scale::Scale;

fn current(facts: &casement_core::output::OutputFacts) -> OutputConfig {
    OutputConfig {
        mode: facts.modes[0],
        scale: Scale(120),
        transform: Transform::Normal,
        position: LogicalPoint { x: 0, y: 0 },
        power: OutputPower::On,
    }
}

#[test]
#[ignore = "P1-M1"]
fn a_change_of_power_keeps_the_rest() {
    let facts = display(
        "DP-1",
        vec![mode(1920, 1080, 60_000, ModePreference::Preferred)],
        (527, 296),
    );
    let config = current(&facts);
    let want = OutputChangeSet::none().with_power(OutputPower::Off);
    let resolved = resolve_change(&facts, &config, &want).expect("resolved");
    assert_eq!(
        resolved,
        OutputConfig {
            power: OutputPower::Off,
            ..config
        }
    );
}

#[test]
#[ignore = "P1-M1"]
fn a_mode_the_display_does_not_offer_is_refused() {
    let facts = display(
        "DP-1",
        vec![mode(1920, 1080, 60_000, ModePreference::Preferred)],
        (527, 296),
    );
    let config = current(&facts);
    let odd = mode(1234, 567, 60_000, ModePreference::Other);
    let mut want = OutputChangeSet::none();
    want.mode = Some(odd);
    assert_eq!(
        resolve_change(&facts, &config, &want),
        Err(OutputRefusal::ModeNotOffered(odd))
    );
}
