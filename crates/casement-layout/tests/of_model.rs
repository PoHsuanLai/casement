//! spec_of reads the model: needs Model::step to build one, so it is proven once P0-F1b-model
//! lands (P0-F1-layout writes spec_of; P0-F1b-model un-ignores this).

use casement_core::app::AppKey;
use casement_core::focus::TokenCheck;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, PhysicalSize};
use casement_core::layer::{
    Anchor, Anchors, ExclusiveZone, KeyboardInteractivity, LayerLevel, LayerSpec, Margins,
    ShellRoleTag,
};
use casement_core::output::{
    EdidFacts, EdidSerial, Mode, ModePreference, OutputFacts, PhysicalMm, VrrCapability,
};
use casement_core::settings::CasementSettings;
use casement_core::space::SpaceSeed;
use casement_core::time::{Millihertz, Mono};
use casement_core::window::{MaxSize, Parent, SizeLimits};
use casement_core::{ClientKey, Event, MappedToplevel, Model, OutputKey, OutputName, WindowId};
use casement_core::{LayerKey, ToplevelRequest};
use casement_layout::{
    LayoutKnobs, RustScene, SceneLayout, SceneMode, SceneSpec, WindowPlacement, spec_of,
};
use casement_protocols::vocab::{Modality, WindowRole};
use porter_core::ClaimedId;
use std::time::Duration;

fn headless(name: &str) -> OutputFacts {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    let name = OutputName::parse(name).expect("name");
    OutputFacts {
        key: OutputKey::of(&edid, &name),
        name,
        edid,
        modes: vec![Mode {
            size: PhysicalSize { w: 1920, h: 1080 },
            refresh: Millihertz(60_000),
            preference: ModePreference::Preferred,
        }],
        physical: PhysicalMm { w: 0, h: 0 },
        vrr: VrrCapability::Incapable,
    }
}

#[test]
fn one_output_one_space_one_window() {
    let at = |ms| Mono(Duration::from_millis(ms));
    let model = Model::new(
        CasementSettings::defaults(),
        at(0),
        SpaceSeed::from_entropy([7; 16]),
    );
    let facts = headless("HEADLESS-1");
    let key = facts.key.clone();
    let (model, _) = model.step(Event::OutputPlugged(facts), at(1));
    let claimed = ClaimedId::parse("foot").expect("claimed");
    let (model, _) = model.step(
        Event::ToplevelMapped(MappedToplevel {
            window: WindowId(1),
            client: ClientKey(1),
            app: AppKey::Claimed(claimed.clone()),
            claimed_id: claimed,
            role: WindowRole::Document,
            modality: Modality::None,
            parent: Parent::None,
            limits: SizeLimits {
                min: LogicalSize { w: 0, h: 0 },
                max: MaxSize::Unbounded,
            },
            first_size: LogicalSize { w: 800, h: 600 },
            activation: TokenCheck::Valid,
        }),
        at(2),
    );
    let spec = spec_of(&model);
    assert_eq!(spec.outputs.len(), 1);
    let output = &spec.outputs[0];
    assert_eq!(output.output, key);
    assert_eq!(
        output.usable,
        LogicalRect::new(
            LogicalPoint { x: 0, y: 0 },
            LogicalSize { w: 1920, h: 1080 }
        )
    );
    let shown = model
        .spaces()
        .shown_on(&key)
        .expect("a plugged output shows a Space")
        .id
        .clone();
    assert_eq!(
        output.mode,
        SceneMode::Desktop {
            shown: shown.clone()
        }
    );
    let holding: Vec<_> = output
        .spaces
        .iter()
        .filter(|s| s.windows.iter().any(|w| w.window == WindowId(1)))
        .map(|s| s.space.clone())
        .collect();
    assert_eq!(holding, vec![shown]);
}

fn at(ms: u64) -> Mono {
    Mono(Duration::from_millis(ms))
}

fn mapped(n: u64, app: &AppKey, token: TokenCheck) -> Event {
    Event::ToplevelMapped(MappedToplevel {
        window: WindowId(n),
        client: ClientKey(n),
        app: app.clone(),
        claimed_id: ClaimedId::parse("foot").expect("claimed"),
        role: WindowRole::Document,
        modality: Modality::None,
        parent: Parent::None,
        limits: SizeLimits {
            min: LogicalSize { w: 0, h: 0 },
            max: MaxSize::Unbounded,
        },
        first_size: LogicalSize { w: 800, h: 600 },
        activation: token,
    })
}

/// The model after `events`, one millisecond apart.
fn model_after(events: Vec<Event>) -> Model {
    let empty = Model::new(
        CasementSettings::defaults(),
        at(0),
        SpaceSeed::from_entropy([7; 16]),
    );
    events
        .into_iter()
        .enumerate()
        .fold(empty, |model, (n, event)| {
            let (model, _) = model.step(event, at(n as u64 + 1));
            assert_eq!(model.check_invariants(), Ok(()), "after event {n}");
            model
        })
}

fn app(name: &str) -> AppKey {
    AppKey::Claimed(ClaimedId::parse(name).expect("claimed"))
}

fn bar(output: &OutputKey, zone: ExclusiveZone) -> LayerSpec {
    LayerSpec {
        output: output.clone(),
        level: LayerLevel::Top,
        anchor: Anchors {
            top: Anchor::Held,
            bottom: Anchor::Free,
            left: Anchor::Held,
            right: Anchor::Held,
        },
        exclusive: zone,
        margin: Margins {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        },
        keyboard: KeyboardInteractivity::None,
        namespace: "bar".to_owned(),
        role: ShellRoleTag::Unassigned,
    }
}

/// The ids of the windows of the output's only Space in the order layout stacks them.
fn stacked(spec: &SceneSpec) -> Vec<u64> {
    spec.outputs[0]
        .spaces
        .iter()
        .flat_map(|space| space.windows.iter().map(|w| w.window.0))
        .collect()
}

#[test]
fn the_usable_area_is_what_the_exclusive_zones_leave() {
    let facts = headless("HEADLESS-1");
    let key = facts.key.clone();
    let model = model_after(vec![
        Event::OutputPlugged(facts),
        Event::LayerMapped {
            layer: LayerKey(1),
            spec: bar(&key, ExclusiveZone::Reserve(28)),
        },
    ]);
    let spec = spec_of(&model);
    assert_eq!(
        spec.outputs[0].usable,
        LogicalRect::new(
            LogicalPoint { x: 0, y: 28 },
            LogicalSize { w: 1920, h: 1052 }
        )
    );
}

#[test]
fn a_new_window_is_centred_in_the_area_the_bar_leaves() {
    let facts = headless("HEADLESS-1");
    let key = facts.key.clone();
    let model = model_after(vec![
        Event::OutputPlugged(facts),
        Event::LayerMapped {
            layer: LayerKey(1),
            spec: bar(&key, ExclusiveZone::Reserve(28)),
        },
        mapped(1, &app("foot"), TokenCheck::Valid),
    ]);
    let spec = spec_of(&model);
    let arrangement = RustScene.arrange(&spec, &LayoutKnobs::of(model.settings()));
    let target = arrangement
        .windows
        .iter()
        .find(|target| target.window == WindowId(1))
        .expect("the window is arranged");
    assert_eq!(
        target.rect,
        LogicalRect::new(
            LogicalPoint {
                x: 560,
                y: 28 + (1052 - 600) / 2
            },
            LogicalSize { w: 800, h: 600 }
        ),
        "centred in what the bar leaves, under the bar"
    );
    assert_eq!(
        target.rect.intersection(&spec.outputs[0].usable),
        Some(target.rect)
    );
}

#[test]
fn a_new_window_is_unplaced_and_the_stack_is_the_models() {
    let facts = headless("HEADLESS-1");
    let foot = app("foot");
    let model = model_after(vec![
        Event::OutputPlugged(facts),
        mapped(1, &foot, TokenCheck::Valid),
        mapped(2, &foot, TokenCheck::Valid),
        mapped(3, &foot, TokenCheck::Valid),
    ]);
    let spec = spec_of(&model);
    assert_eq!(stacked(&spec), vec![1, 2, 3]);
    let placements: Vec<WindowPlacement> = spec.outputs[0].spaces[0]
        .windows
        .iter()
        .map(|w| w.placement.clone())
        .collect();
    assert_eq!(
        placements,
        vec![WindowPlacement::New(LogicalSize { w: 800, h: 600 }); 3]
    );

    let raised = model_after(vec![
        Event::OutputPlugged(headless("HEADLESS-1")),
        mapped(1, &foot, TokenCheck::Valid),
        mapped(2, &foot, TokenCheck::Valid),
        mapped(3, &foot, TokenCheck::Valid),
        Event::ToplevelRequest {
            window: WindowId(1),
            request: ToplevelRequest::Activate(TokenCheck::Valid),
        },
    ]);
    let order = stacked(&spec_of(&raised));
    assert_eq!(order, vec![2, 3, 1], "activating window 1 put it on top");
    let indexes: Vec<u32> = spec_of(&raised).outputs[0].spaces[0]
        .windows
        .iter()
        .map(|w| w.stack.0)
        .collect();
    assert_eq!(indexes, vec![0, 1, 2]);
}

#[test]
fn a_hidden_apps_windows_and_minimised_ones_are_not_on_the_desktop() {
    let (foot, web) = (app("foot"), app("web"));
    let model = model_after(vec![
        Event::OutputPlugged(headless("HEADLESS-1")),
        mapped(1, &foot, TokenCheck::Valid),
        mapped(2, &web, TokenCheck::Valid),
        Event::ToplevelRequest {
            window: WindowId(1),
            request: ToplevelRequest::Minimise,
        },
    ]);
    let hidden = model
        .clone()
        .command(casement_core::input::DesktopCommand::HideActive, at(10))
        .0;
    for (name, model) in [("minimised", &model), ("hidden", &hidden)] {
        let spec = spec_of(model);
        let windows = &spec.outputs[0].spaces[0].windows;
        let placement = |n: u64| {
            windows
                .iter()
                .find(|w| w.window == WindowId(n))
                .map(|w| w.placement.clone())
        };
        let gone = if name == "minimised" { 1 } else { 2 };
        assert_eq!(placement(gone), Some(WindowPlacement::Minimised), "{name}");
    }
}

#[test]
fn a_window_on_every_space_is_arranged_on_its_home_outputs_shown_space_only() {
    let bar_app = app("bar");
    let shell = Event::ToplevelMapped(MappedToplevel {
        role: WindowRole::Shell,
        ..match mapped(1, &bar_app, TokenCheck::Missing) {
            Event::ToplevelMapped(mapped) => mapped,
            other => unreachable!("{other:?}"),
        }
    });
    let model = model_after(vec![
        Event::OutputPlugged(headless("HEADLESS-1")),
        Event::OutputPlugged(headless("HEADLESS-2")),
        shell,
    ]);
    let spec = spec_of(&model);
    assert_eq!(spec.outputs.len(), 2);
    let holding: Vec<(usize, usize)> = spec
        .outputs
        .iter()
        .enumerate()
        .flat_map(|(o, output)| {
            output
                .spaces
                .iter()
                .enumerate()
                .filter(|(_, space)| space.windows.iter().any(|w| w.window == WindowId(1)))
                .map(move |(s, _)| (o, s))
        })
        .collect();
    assert_eq!(
        holding,
        vec![(0, 0)],
        "once, on the first output's shown Space"
    );
}

#[test]
fn an_output_with_no_window_is_still_in_the_scene() {
    let model = model_after(vec![
        Event::OutputPlugged(headless("HEADLESS-1")),
        Event::OutputPlugged(headless("HEADLESS-2")),
    ]);
    let spec = spec_of(&model);
    assert_eq!(spec.outputs.len(), 2);
    assert!(spec.outputs.iter().all(|output| output.spaces.len() == 1));
}
