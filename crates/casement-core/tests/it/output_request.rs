//! A client's output change request, resolved against the display's facts (P1-M1), and what
//! the model does with one.

use crate::support::driver::{plug, run};
use crate::support::{display, headless, mode};
use casement_core::geometry::{LogicalPoint, Transform};
use casement_core::output::{
    ModePreference, OutputChangeSet, OutputConfig, OutputPower, OutputRefusal, resolve_change,
};
use casement_core::{Effect, Event};
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

#[test]
fn a_scale_outside_what_outputs_take_is_refused() {
    let facts = display(
        "DP-1",
        vec![mode(1920, 1080, 60_000, ModePreference::Preferred)],
        (527, 296),
    );
    let config = current(&facts);
    for numerator in [0, 29, 961] {
        let mut want = OutputChangeSet::none();
        want.scale = Some(Scale(numerator));
        assert_eq!(
            resolve_change(&facts, &config, &want),
            Err(OutputRefusal::ScaleOutOfRange(Scale(numerator)))
        );
    }
}

#[test]
fn scale_transform_and_position_replace_only_themselves() {
    let facts = display(
        "DP-1",
        vec![mode(1920, 1080, 60_000, ModePreference::Preferred)],
        (527, 296),
    );
    let config = current(&facts);
    let mut want = OutputChangeSet::none();
    want.scale = Some(Scale(180));
    want.transform = Some(Transform::R90);
    want.position = Some(LogicalPoint { x: 1920, y: -40 });
    let resolved = resolve_change(&facts, &config, &want).expect("resolved");
    assert_eq!(
        resolved,
        OutputConfig {
            scale: Scale(180),
            transform: Transform::R90,
            position: LogicalPoint { x: 1920, y: -40 },
            ..config
        }
    );
}

fn requested(change: OutputChangeSet) -> (casement_core::Model, Vec<Effect>) {
    let plugged = run(vec![plug("HEADLESS-1")]);
    let key = headless("HEADLESS-1").key;
    let at = plugged
        .model
        .now()
        .after(std::time::Duration::from_millis(1));
    plugged.model.step(
        Event::OutputRequested {
            output: key,
            change,
        },
        at,
    )
}

#[test]
fn the_model_applies_an_accepted_request_and_keeps_the_new_configuration() {
    let key = headless("HEADLESS-1").key;
    let mut want = OutputChangeSet::none();
    want.scale = Some(Scale(240));
    let (model, effects) = requested(want);
    let config = model.outputs().0[&key].config;
    assert_eq!(config.scale, Scale(240));
    assert!(effects.contains(&Effect::ApplyOutput {
        output: key,
        config
    }));
}

#[test]
fn the_model_notes_a_refused_request_and_changes_nothing() {
    let key = headless("HEADLESS-1").key;
    let before = run(vec![plug("HEADLESS-1")]).model.outputs().0[&key].config;
    let mut want = OutputChangeSet::none();
    want.mode = Some(mode(1234, 567, 60_000, ModePreference::Other));
    let (model, effects) = requested(want);
    assert_eq!(model.outputs().0[&key].config, before);
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::Note(casement_core::CoreNote::RefusedOutput(_))))
    );
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::ApplyOutput { .. }))
    );
}

#[test]
fn gamma_for_a_plugged_output_becomes_an_effect_and_for_a_gone_one_nothing() {
    let key = headless("HEADLESS-1").key;
    let plugged = run(vec![plug("HEADLESS-1")]);
    let at = plugged
        .model
        .now()
        .after(std::time::Duration::from_millis(1));
    let ramp = casement_core::output::GammaRamp::Identity;
    let (model, effects) = plugged.model.step(
        Event::GammaSet {
            output: key.clone(),
            ramp: ramp.clone(),
        },
        at,
    );
    assert_eq!(effects, vec![Effect::SetGamma(key, ramp.clone())]);
    let gone = headless("HEADLESS-9").key;
    let at = model.now().after(std::time::Duration::from_millis(1));
    let (_, effects) = model.step(Event::GammaSet { output: gone, ramp }, at);
    assert!(effects.is_empty());
}
