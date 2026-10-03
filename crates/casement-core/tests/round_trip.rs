//! Every stored or wire type survives its serde form (CONVENTIONS §12), with the forms pinned.

mod support;

use casement_core::app::AppKey;
use casement_core::geometry::{LogicalPoint, Transform};
use casement_core::input::{
    Argv, Binding, BindingAction, Bindings, Chord, DbusCall, DesktopCommand, DeviceClass,
    FeedSource, Keysym, Origin, PortalSessionId, SpaceStep,
};
use casement_core::output::{
    EdidFacts, EdidSerial, ModePreference, OutputConfig, OutputPower, SavedOutputs, ScalePolicy,
};
use casement_core::space::{Space, SpaceContent, SpaceIndex, SplitPair, TaskToken};
use casement_core::{LeaseId, WindowId};
use casement_protocols::vocab::{MacMods, PlaceTarget};
use ds_core::geometry::scale::Scale;
use ds_core::vocab::Fraction;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use support::{claimed, headless, mode, proven};

/// `value`'s JSON is `json`, and reads back as `value`.
fn pinned<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T, json: &str) {
    let text = serde_json::to_string(value).expect("serialise");
    assert_eq!(text, json);
    assert_eq!(&serde_json::from_str::<T>(&text).expect("parse"), value);
}

#[test]
fn app_keys_and_spaces() {
    pinned(
        &AppKey::Claimed(claimed("foot")),
        r#"{"kind":"claimed","v":"foot"}"#,
    );
    pinned(
        &proven("org.quire.Mail"),
        r#"{"kind":"proven","v":{"name":"org.quire.Mail","isolation":"unsandboxed"}}"#,
    );
    let output = headless("HEADLESS-1").key;
    let space = Space {
        id: support::space_id("s-0a"),
        output,
        content: SpaceContent::SplitView(SplitPair {
            leading: WindowId(1),
            trailing: WindowId(2),
            divider: Fraction(600),
        }),
        position: SpaceIndex(2),
    };
    pinned(
        &space,
        r#"{"id":"s-0a","output":"connector:HEADLESS-1","content":{"kind":"split_view","v":{"leading":1,"trailing":2,"divider":600}},"position":2}"#,
    );
    pinned(
        &SpaceContent::Agent(TaskToken("task-7".to_owned())),
        r#"{"kind":"agent","v":"task-7"}"#,
    );
    pinned(&SpaceContent::Desktop, r#"{"kind":"desktop"}"#);
}

#[test]
fn origins() {
    pinned(
        &Origin::Physical(DeviceClass::Touchpad),
        r#"{"kind":"physical","v":"touchpad"}"#,
    );
    pinned(
        &Origin::Virtual(DeviceClass::Keyboard),
        r#"{"kind":"virtual","v":"keyboard"}"#,
    );
    pinned(
        &Origin::Feed(FeedSource::Palmrest),
        r#"{"kind":"feed","v":"palmrest"}"#,
    );
    pinned(&Origin::Agent(LeaseId(4)), r#"{"kind":"agent","v":4}"#);
    pinned(
        &Origin::Remote(PortalSessionId(5)),
        r#"{"kind":"remote","v":5}"#,
    );
    pinned(&Origin::Test, r#"{"kind":"test"}"#);
}

#[test]
fn binding_actions() {
    pinned(
        &BindingAction::Call(DbusCall {
            service: "org.quire.Shell".to_owned(),
            path: "/org/quire/Shell".to_owned(),
            interface: "org.quire.Shell1".to_owned(),
            method: "ToggleLauncher".to_owned(),
            args: Vec::new(),
        }),
        r#"{"kind":"call","v":{"service":"org.quire.Shell","path":"/org/quire/Shell","interface":"org.quire.Shell1","method":"ToggleLauncher","args":[]}}"#,
    );
    pinned(
        &BindingAction::Spawn(Argv(vec!["foot".to_owned()])),
        r#"{"kind":"spawn","v":["foot"]}"#,
    );
    pinned(
        &BindingAction::Desktop(DesktopCommand::SwitchSpace(SpaceStep::Number(3))),
        r#"{"kind":"desktop","v":{"kind":"switch_space","v":{"kind":"number","v":3}}}"#,
    );
    pinned(
        &BindingAction::Desktop(DesktopCommand::Place(PlaceTarget::TopLeft)),
        r#"{"kind":"desktop","v":{"kind":"place","v":"top_left"}}"#,
    );
}

#[test]
fn outputs() {
    pinned(
        &EdidFacts {
            make: "Dell".to_owned(),
            model: "U2723QE".to_owned(),
            serial: EdidSerial::Known("7XK".to_owned()),
        },
        r#"{"make":"Dell","model":"U2723QE","serial":{"kind":"known","v":"7XK"}}"#,
    );
    let config = OutputConfig {
        mode: mode(3840, 2160, 59_997, ModePreference::Preferred),
        scale: Scale(240),
        transform: Transform::R90,
        position: LogicalPoint { x: -1920, y: 0 },
        power: OutputPower::On,
    };
    pinned(
        &config,
        r#"{"mode":{"size":{"w":3840,"h":2160},"refresh":59997,"preference":"preferred"},"scale":240,"transform":"r90","position":{"x":-1920,"y":0},"power":"on"}"#,
    );
    pinned(&ScalePolicy::WholeLikeMac, r#"{"kind":"whole_like_mac"}"#);
    pinned(
        &ScalePolicy::Fixed(Scale(180)),
        r#"{"kind":"fixed","v":180}"#,
    );
    let saved = SavedOutputs(BTreeMap::from([(headless("HEADLESS-1").key, config)]));
    pinned(
        &saved,
        &format!(
            r#"{{"connector:HEADLESS-1":{}}}"#,
            serde_json::to_string(&config).expect("json")
        ),
    );
    let file = toml::to_string(&saved).expect("toml");
    assert_eq!(
        toml::from_str::<SavedOutputs>(&file).expect("toml back"),
        saved
    );
}

#[test]
#[ignore = "P0-F1-input"]
fn chords_and_bindings_store_as_text() {
    let chord = Chord {
        mods: MacMods::COMMAND,
        key: Keysym(0x20),
    };
    pinned(&chord, r#""cmd+space""#);
    let bindings = Bindings(vec![Binding {
        chord,
        action: BindingAction::Desktop(DesktopCommand::HideActive),
    }]);
    pinned(
        &bindings,
        r#"[{"chord":"cmd+space","action":{"kind":"desktop","v":{"kind":"hide_active"}}}]"#,
    );
    assert!(serde_json::from_str::<Chord>(r#""cmd+""#).is_err());
}
