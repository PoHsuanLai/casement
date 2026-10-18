//! A client's output change request, resolved against the display's facts (P1-M1), and what
//! the model does with one.

use crate::support::driver::{Op, plug, run, run_from};
use crate::support::{display, headless, mode, ms};
use casement_core::geometry::{LogicalPoint, Transform};
use casement_core::output::{
    GammaRamp, GammaTable, Kelvin, MinuteOfDay, ModePreference, NightLight, OutputChangeSet,
    OutputConfig, OutputPower, OutputRefusal, Requester, Schedule, keeps_one_lit, resolve_change,
};
use casement_core::settings::{CasementSettings, NightLightSettings};
use casement_core::space::SpaceSeed;
use casement_core::{Effect, Event, Model, OutputKey};
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
            by: Requester::Shell,
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

fn refused(effects: &[Effect]) -> bool {
    effects
        .iter()
        .any(|e| matches!(e, Effect::Note(casement_core::CoreNote::RefusedOutput(_))))
}

/// CASC-09: a request cannot put an output where the pointer never goes.
#[test]
fn a_position_far_outside_any_arrangement_is_refused() {
    let facts = display(
        "DP-1",
        vec![mode(1920, 1080, 60_000, ModePreference::Preferred)],
        (527, 296),
    );
    let config = current(&facts);
    let cases = [
        ("far right", LogicalPoint { x: i32::MAX, y: 0 }, false),
        ("far up", LogicalPoint { x: 0, y: i32::MIN }, false),
        ("beside", LogicalPoint { x: 1920, y: -40 }, true),
        ("a long row", LogicalPoint { x: 100_000, y: 0 }, true),
    ];
    for (name, at, fine) in cases {
        let mut want = OutputChangeSet::none();
        want.position = Some(at);
        let resolved = resolve_change(&facts, &config, &want);
        match fine {
            true => assert!(resolved.is_ok(), "{name}"),
            false => assert_eq!(
                resolved,
                Err(OutputRefusal::PositionOutOfRange(at)),
                "{name}"
            ),
        }
    }
}

/// CASC-09: the last lit display cannot be turned dark by a request; one of two can, and then
/// the other cannot.
#[test]
fn a_request_cannot_darken_the_last_lit_output() {
    let (a, b) = (headless("HEADLESS-1").key, headless("HEADLESS-2").key);
    let dark = OutputChangeSet::none().with_power(OutputPower::Off);
    let ask = |output: &OutputKey| {
        Op::Event(Event::OutputRequested {
            output: output.clone(),
            change: dark,
            by: Requester::Shell,
        })
    };
    let alone = run(vec![plug("HEADLESS-1"), ask(&a)]);
    assert_eq!(alone.model.outputs().0[&a].config.power, OutputPower::On);
    assert!(refused(alone.effects.last().expect("a step")));

    let two = run(vec![plug("HEADLESS-1"), plug("HEADLESS-2"), ask(&a)]);
    assert_eq!(two.model.outputs().0[&a].config.power, OutputPower::Off);
    assert!(!refused(two.effects.last().expect("a step")));
    let both = run_from(two.model, vec![ask(&b)]);
    assert_eq!(both.model.outputs().0[&b].config.power, OutputPower::On);
    assert!(refused(both.effects.last().expect("a step")));

    // The same rule over a whole configuration.
    let book = both.model.outputs();
    let all_dark = [(a.clone(), dark), (b.clone(), dark)];
    assert_eq!(
        keeps_one_lit(book, &all_dark),
        Err(OutputRefusal::NoneLeftLit)
    );
    let swap = [
        (
            a.clone(),
            OutputChangeSet::none().with_power(OutputPower::On),
        ),
        (b, dark),
    ];
    assert_eq!(keeps_one_lit(book, &swap), Ok(()));
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

/// Night light all day (no fade) leaving `skipping` alone.
fn night(skipping: &[&str]) -> CasementSettings {
    let light = NightLightSettings::new(NightLight {
        schedule: Schedule::Always,
        warmth: Kelvin::NIGHT,
        fade_minutes: 0,
    })
    .skipping(skipping.iter().map(|name| (*name).to_owned()).collect());
    CasementSettings {
        night_light: light,
        ..CasementSettings::defaults()
    }
}

fn night_model(skipping: &[&str]) -> Model {
    Model::new(night(skipping), ms(0), SpaceSeed::from_entropy([9; 16]))
}

fn tick(minute: u16) -> Op {
    Op::Event(Event::NightLightTick(MinuteOfDay::wrapping(minute)))
}

fn settings(skipping: &[&str]) -> Op {
    Op::Event(Event::SettingsChanged(Box::new(night(skipping))))
}

/// The gamma ramps one step asked for, by output.
fn ramps(effects: &[Effect]) -> Vec<(OutputKey, GammaRamp)> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::SetGamma(key, ramp) => Some((key.clone(), ramp.clone())),
            _ => None,
        })
        .collect()
}

fn is_table(ramp: &GammaRamp) -> bool {
    matches!(ramp, GammaRamp::Table(_))
}

#[test]
fn night_light_warms_each_output_it_is_not_told_to_leave_alone_once() {
    let (one, two) = (headless("HEADLESS-1").key, headless("HEADLESS-2").key);
    let ops = vec![plug("HEADLESS-1"), plug("HEADLESS-2"), tick(720), tick(721)];
    let run = run_from(night_model(&["HEADLESS-2"]), ops);
    assert!(
        run.effects[..2].iter().all(|step| ramps(step).is_empty()),
        "nothing is driven before the first tick"
    );
    let first = ramps(&run.effects[2]);
    assert_eq!(
        first.len(),
        1,
        "the skipped output is not touched: {first:?}"
    );
    assert_eq!(first[0].0, one);
    assert!(is_table(&first[0].1));
    assert_ne!(first[0].0, two);
    assert!(
        ramps(&run.effects[3]).is_empty(),
        "a steady minute asks nothing"
    );
}

#[test]
fn a_changed_skip_list_takes_effect_at_once_and_a_held_output_is_left_to_its_client() {
    let one = headless("HEADLESS-1").key;
    let client = GammaTable::new(vec![0, 65535], vec![0, 65535], vec![0, 65535]).expect("a table");
    let ops = vec![
        plug("HEADLESS-1"),
        tick(0),
        settings(&["HEADLESS-1"]),
        settings(&[]),
        Op::Event(Event::GammaSet {
            output: one.clone(),
            ramp: GammaRamp::Table(client.clone()),
        }),
        tick(1),
        Op::Event(Event::GammaSet {
            output: one.clone(),
            ramp: GammaRamp::Identity,
        }),
    ];
    let run = run_from(night_model(&[]), ops);
    assert!(is_table(&ramps(&run.effects[1])[0].1), "warmed at the tick");
    assert_eq!(
        ramps(&run.effects[2]),
        vec![(one.clone(), GammaRamp::Identity)],
        "listed: back to its own curve"
    );
    let rewarmed = ramps(&run.effects[3]);
    assert_eq!(rewarmed.len(), 1);
    assert!(is_table(&rewarmed[0].1), "unlisted: warmed again");
    assert_eq!(
        ramps(&run.effects[4]),
        vec![(one.clone(), GammaRamp::Table(client))],
        "the client's table goes through"
    );
    assert!(
        ramps(&run.effects[5]).is_empty(),
        "the client holds the output"
    );
    let released = ramps(&run.effects[6]);
    assert_eq!(
        released.len(),
        2,
        "its identity, then the night light again"
    );
    assert_eq!(released[0].1, GammaRamp::Identity);
    assert!(is_table(&released[1].1));
}

/// CASW-18: a client's ramp is raised to an eighth of the identity curve and not touched where
/// it is already above it.
#[test]
fn a_ramp_is_floored_at_an_eighth_of_the_identity_curve() {
    let black = GammaTable::new(vec![0; 256], vec![0; 256], vec![0; 256])
        .expect("a table")
        .floored();
    assert_eq!(black.red()[0], 0);
    assert_eq!(black.red()[255], 65535 / 8);
    assert!(black.red().windows(2).all(|pair| pair[0] <= pair[1]));
    let bright = GammaTable::new(vec![65535; 4], vec![40_000; 4], vec![65535; 4]).expect("a table");
    assert_eq!(bright.clone().floored(), bright);
}
