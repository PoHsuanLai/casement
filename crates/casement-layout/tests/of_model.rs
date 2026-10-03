//! spec_of reads the model: needs Model::step to build one, so it is proven once P0-F1b-model
//! lands (P0-F1-layout writes spec_of; P0-F1b-model un-ignores this).

use casement_core::app::AppKey;
use casement_core::focus::TokenCheck;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, PhysicalSize};
use casement_core::output::{
    EdidFacts, EdidSerial, Mode, ModePreference, OutputFacts, PhysicalMm, VrrCapability,
};
use casement_core::settings::CasementSettings;
use casement_core::space::SpaceSeed;
use casement_core::time::{Millihertz, Mono};
use casement_core::window::{MaxSize, Parent, SizeLimits};
use casement_core::{ClientKey, Event, MappedToplevel, Model, OutputKey, OutputName, WindowId};
use casement_layout::{SceneMode, spec_of};
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
#[ignore = "P0-F1b-model"]
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
