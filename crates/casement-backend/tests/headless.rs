//! The headless backend's outputs: plug, unplug, change and apply report the same
//! `ConnectorEvent`s a KMS connector would, and nothing is left behind (P0-B-headless).

use casement_backend::{
    ConnectorEvent, DmabufMode, HeadlessBackend, HeadlessConfig, OutputChange, RenderCostModel,
    SoftRenderer, VirtualOutputSpec,
};
use casement_core::geometry::{LogicalPoint, Transform};
use casement_core::output::{OutputConfig, OutputPower};
use casement_core::{OutputKey, OutputName};
use ds_core::geometry::scale::Scale;
use std::time::Duration;

fn spec(text: &str) -> VirtualOutputSpec {
    VirtualOutputSpec::parse(text).expect(text)
}

fn name(text: &str) -> OutputName {
    OutputName::parse(text).expect(text)
}

fn config(outputs: &[&str]) -> HeadlessConfig {
    HeadlessConfig {
        outputs: outputs.iter().map(|text| spec(text)).collect(),
        renderer: SoftRenderer::Pixman,
        dmabuf: DmabufMode::None,
        cost: RenderCostModel::Fixed(Duration::from_millis(4)),
    }
}

fn key_of(event: &ConnectorEvent) -> OutputKey {
    match event {
        ConnectorEvent::Plugged(facts) | ConnectorEvent::Changed(facts) => facts.key.clone(),
        ConnectorEvent::Unplugged(key) => key.clone(),
    }
}

#[test]
fn the_outputs_plugged_at_start_are_announced_once_in_name_order() {
    let mut backend = HeadlessBackend::start(config(&[
        "HEADLESS-2:2880x1620@60000:180:1920,0",
        "HEADLESS-1:1920x1080@60000:120:0,0",
        "HEADLESS-3:3840x2160@60000:240:3840,0",
    ]))
    .expect("starts");
    assert_eq!(backend.outputs().len(), 3);
    let announced = backend.announce();
    assert_eq!(announced.len(), 3);
    assert!(
        announced
            .iter()
            .all(|e| matches!(e, ConnectorEvent::Plugged(_)))
    );
    assert!(backend.announce().is_empty(), "announced once");
    assert!(
        backend.last_frame(&name("HEADLESS-1")).is_none(),
        "nothing has presented before a frame"
    );
    assert_eq!(backend.frames_presented(), 0);
    assert_eq!(backend.callbacks_sent(), 0);
}

#[test]
fn a_plug_reports_the_facts_of_the_spec() {
    let mut backend = HeadlessBackend::start(config(&[])).expect("starts");
    let event = backend.plug(spec("DP-1:3840x2160@60000:180:0,0:vrr=capable"));
    let ConnectorEvent::Plugged(facts) = event else {
        panic!("a plug is Plugged");
    };
    assert_eq!(facts.name, name("DP-1"));
    assert_eq!(
        facts.key.as_str(),
        "connector:DP-1",
        "no serial: the key is the connector"
    );
    assert_eq!(facts.modes.len(), 1);
    assert_eq!(facts.modes[0].size.w, 3840);
    assert_eq!(facts.modes[0].refresh.0, 60_000);
}

#[test]
fn a_recorded_edid_gives_a_key_that_survives_a_new_connector() {
    let mut backend = HeadlessBackend::start(config(&[])).expect("starts");
    let first = backend.plug(spec("DP-1:1920x1080@60000:120:0,0:edid=framework-13"));
    backend.unplug(&name("DP-1"));
    let second = backend.plug(spec("DP-2:1920x1080@60000:120:0,0:edid=framework-13"));
    assert_eq!(key_of(&first), key_of(&second));
}

#[test]
fn plugging_a_connector_twice_replaces_it() {
    let mut backend =
        HeadlessBackend::start(config(&["HEADLESS-1:1920x1080@60000:120:0,0"])).expect("starts");
    let event = backend.plug(spec("HEADLESS-1:2560x1440@60000:120:0,0"));
    assert!(matches!(event, ConnectorEvent::Changed(_)));
    assert_eq!(backend.outputs().len(), 1);
    let plugged = backend.outputs().get(&name("HEADLESS-1")).expect("plugged");
    assert_eq!(plugged.spec.mode.size.w, 2560);
}

#[test]
fn an_unplug_reports_the_key_it_plugged_with() {
    let mut backend = HeadlessBackend::start(config(&[])).expect("starts");
    let plugged = backend.plug(spec("HEADLESS-1:1920x1080@60000:120:0,0"));
    let unplugged = backend.unplug(&name("HEADLESS-1"));
    assert_eq!(unplugged, ConnectorEvent::Unplugged(key_of(&plugged)));
    assert!(backend.outputs().is_empty());
}

#[test]
fn what_is_not_plugged_unplugs_and_changes_to_nothing() {
    let mut backend = HeadlessBackend::start(config(&[])).expect("starts");
    let unplugged = backend.unplug(&name("HEADLESS-7"));
    let changed = backend.set(&name("HEADLESS-7"), OutputChange::Scale(Scale(180)));
    assert!(matches!(unplugged, ConnectorEvent::Unplugged(_)));
    assert_eq!(changed, unplugged);
}

#[test]
fn a_change_updates_the_output_and_reports_it() {
    let mut backend =
        HeadlessBackend::start(config(&["HEADLESS-1:1920x1080@60000:120:0,0"])).expect("starts");
    let target = name("HEADLESS-1");
    let to = |backend: &HeadlessBackend| {
        backend
            .outputs()
            .get(&target)
            .expect("plugged")
            .spec
            .clone()
    };
    let event = backend.set(&target, OutputChange::Scale(Scale(180)));
    assert!(matches!(event, ConnectorEvent::Changed(_)));
    assert_eq!(to(&backend).scale, Scale(180));
    backend.set(
        &target,
        OutputChange::Position(LogicalPoint { x: 40, y: 20 }),
    );
    assert_eq!(to(&backend).position, LogicalPoint { x: 40, y: 20 });
    let mut mode = to(&backend).mode;
    mode.size.w = 1280;
    mode.size.h = 720;
    backend.set(&target, OutputChange::Mode(mode));
    assert_eq!(to(&backend).mode.size.w, 1280);
    backend.set(&target, OutputChange::Power(OutputPower::Off));
    assert_eq!(
        backend.outputs().get(&target).expect("plugged").power,
        OutputPower::Off
    );
}

#[test]
fn apply_sets_the_whole_configuration_at_once() {
    let mut backend =
        HeadlessBackend::start(config(&["HEADLESS-1:1920x1080@60000:120:0,0"])).expect("starts");
    let target = name("HEADLESS-1");
    let mut mode = backend.outputs().get(&target).expect("plugged").spec.mode;
    mode.size.w = 2560;
    mode.size.h = 1440;
    let wanted = OutputConfig {
        mode,
        scale: Scale(150),
        transform: Transform::R90,
        position: LogicalPoint { x: -2560, y: 0 },
        power: OutputPower::Off,
    };
    assert!(matches!(
        backend.apply(&target, &wanted),
        ConnectorEvent::Changed(_)
    ));
    let output = backend.outputs().get(&target).expect("plugged");
    assert_eq!(output.spec.mode, mode);
    assert_eq!(output.spec.scale, Scale(150));
    assert_eq!(output.spec.transform, Transform::R90);
    assert_eq!(output.spec.position, wanted.position);
    assert_eq!(output.power, OutputPower::Off);
}

#[test]
fn the_dummy_renderer_needs_the_testing_feature_and_the_others_start() {
    let mut pixman = config(&[]);
    pixman.renderer = SoftRenderer::Pixman;
    assert!(HeadlessBackend::start(pixman).is_ok());
    let mut dummy = config(&[]);
    dummy.renderer = SoftRenderer::Dummy;
    assert_eq!(
        HeadlessBackend::start(dummy).is_ok(),
        cfg!(feature = "testing")
    );
}
