//! The settings document (P0-F1-output): defaults when absent, one conversion into the model's
//! settings, and the schema detent reads (`casement --write-schema`). Every document lives in a
//! scratch directory standing in for `$XDG_CONFIG_HOME`; nothing here reads the real one.

use casement::cli::ConfigHome;
use casement::config::{
    ActionWords, BindingsSection, CasementDoc, load, schema, to_core, write_schema,
};
use casement_core::input::{
    Argv, Binding, BindingAction, Bindings, Chord, DbusCall, DesktopCommand, SpaceStep,
};
use casement_core::output::ScalePolicy;
use casement_core::settings::{
    CasementSettings, Enabled, FirstClick, MinimiseEffect, RepeatRate, TitleDoubleClick,
};
use casement_protocols::vocab::PlaceTarget;
use ds_core::geometry::scale::Scale;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// A scratch config home holding `text` as `casement/settings.toml`.
fn home_with(text: &str) -> (tempfile::TempDir, ConfigHome) {
    let dir = tempfile::tempdir().expect("temp dir");
    let casement = dir.path().join("casement");
    std::fs::create_dir_all(&casement).expect("create");
    std::fs::write(casement.join("settings.toml"), text).expect("write");
    let home = ConfigHome::At(dir.path().to_path_buf());
    (dir, home)
}

fn settings_of(text: &str) -> CasementSettings {
    let (_dir, home) = home_with(text);
    to_core(&load(&home).expect("load"))
}

#[test]
fn a_missing_document_gives_the_defaults() {
    let home = tempfile::tempdir().expect("temp dir");
    let doc = load(&ConfigHome::At(home.path().to_path_buf())).expect("load");
    assert_eq!(doc, CasementDoc::default());
    assert_eq!(to_core(&doc), CasementSettings::defaults());
}

#[test]
fn a_directory_that_does_not_exist_gives_the_defaults() {
    let home = ConfigHome::At(Path::new("/nonexistent/casement-test-home").to_path_buf());
    assert_eq!(
        to_core(&load(&home).expect("load")),
        CasementSettings::defaults()
    );
}

#[test]
fn an_empty_document_gives_the_defaults() {
    assert_eq!(settings_of(""), CasementSettings::defaults());
}

#[test]
fn the_defaults_survive_a_round_trip_through_the_document() {
    let text = toml::to_string(&CasementDoc::default()).expect("serialises");
    assert_eq!(settings_of(&text), CasementSettings::defaults());
}

#[test]
fn the_keys_a_document_sets_reach_the_model() {
    let settings = settings_of(
        r#"
[windows]
minimise_effect = "scale"
double_click_title = "fill"
tile_margins = "off"
first_click = "activate_and_click"
quit_grace_ms = 500
not_responding_ms = 8000

[spaces]
per_display = "off"
auto_rearrange = "on"

[keyboard]
xkb_layout = "us,de"
xkb_variant = ",nodeadkeys"
xkb_options = "caps:escape"
repeat_delay_ms = 250
repeat_rate = 30

[outputs]
first_sight_scale = "fixed"
fixed_scale = 150
"#,
    );
    let defaults = CasementSettings::defaults();
    assert_eq!(settings.windows.minimise_effect, MinimiseEffect::Scale);
    assert_eq!(settings.windows.double_click_title, TitleDoubleClick::Fill);
    assert_eq!(settings.windows.tile_margins, Enabled::Off);
    assert_eq!(settings.windows.tile_edges, defaults.windows.tile_edges);
    assert_eq!(settings.windows.first_click, FirstClick::ActivateAndClick);
    assert_eq!(settings.windows.quit_grace, Duration::from_millis(500));
    assert_eq!(settings.windows.not_responding, Duration::from_millis(8000));
    assert_eq!(settings.spaces.per_display, Enabled::Off);
    assert_eq!(settings.spaces.auto_rearrange, Enabled::On);
    assert_eq!(
        settings.spaces.switch_on_activate,
        defaults.spaces.switch_on_activate
    );
    assert_eq!(settings.keyboard.layout.layout, "us,de");
    assert_eq!(settings.keyboard.layout.variant, ",nodeadkeys");
    assert_eq!(settings.keyboard.layout.options, "caps:escape");
    assert_eq!(settings.keyboard.layout.rules, "");
    assert_eq!(settings.keyboard.repeat_delay, Duration::from_millis(250));
    assert_eq!(settings.keyboard.repeat_rate, RepeatRate(30));
    assert_eq!(settings.outputs.first_sight, ScalePolicy::Fixed(Scale(180)));
}

#[test]
fn a_bad_value_costs_only_its_own_key() {
    let settings = settings_of(
        r#"
[windows]
minimise_effect = "explode"
tile_edges = "off"

[keyboard]
repeat_rate = "fast"
repeat_delay_ms = 300
"#,
    );
    let defaults = CasementSettings::defaults();
    assert_eq!(
        settings.windows.minimise_effect,
        defaults.windows.minimise_effect
    );
    assert_eq!(settings.windows.tile_edges, Enabled::Off);
    assert_eq!(settings.keyboard.repeat_rate, defaults.keyboard.repeat_rate);
    assert_eq!(settings.keyboard.repeat_delay, Duration::from_millis(300));
}

#[test]
fn a_document_that_is_not_toml_gives_the_defaults() {
    assert_eq!(
        settings_of("[windows\nminimise_effect = "),
        CasementSettings::defaults()
    );
}

#[test]
fn an_unknown_key_is_ignored() {
    assert_eq!(
        settings_of("[windows]\nbounce = 3\n[dock]\nsize = 1\n"),
        CasementSettings::defaults()
    );
}

/// Each row: what the document says about `outputs`, and the policy the model gets.
#[test]
fn the_scale_policy_follows_the_document() {
    let cases = [
        ("nothing", "", ScalePolicy::WholeLikeMac),
        (
            "the Mac's policy named",
            "first_sight_scale = \"whole_like_mac\"",
            ScalePolicy::WholeLikeMac,
        ),
        (
            "a fixed scale is read as a percent",
            "first_sight_scale = \"fixed\"\nfixed_scale = 200",
            ScalePolicy::Fixed(Scale(240)),
        ),
        (
            "a fixed scale without a number is 1x",
            "first_sight_scale = \"fixed\"",
            ScalePolicy::Fixed(Scale(120)),
        ),
        (
            "a fixed scale is kept in its range",
            "first_sight_scale = \"fixed\"\nfixed_scale = 5",
            ScalePolicy::Fixed(Scale(120)),
        ),
        (
            "and not above it",
            "first_sight_scale = \"fixed\"\nfixed_scale = 900",
            ScalePolicy::Fixed(Scale(480)),
        ),
        (
            "a fixed scale ignored under the Mac policy",
            "first_sight_scale = \"whole_like_mac\"\nfixed_scale = 300",
            ScalePolicy::WholeLikeMac,
        ),
    ];
    for (name, outputs, want) in cases {
        let settings = settings_of(&format!("[outputs]\n{outputs}\n"));
        assert_eq!(settings.outputs.first_sight, want, "{name}");
    }
}

#[test]
fn a_key_repeat_that_would_never_fire_is_one_a_second() {
    let settings = settings_of("[keyboard]\nrepeat_rate = 0\n");
    assert_eq!(settings.keyboard.repeat_rate, RepeatRate(1));
}

#[test]
fn bindings_are_chord_text_to_action() {
    let settings = settings_of(
        r#"
[bindings]
"cmd+q" = ["desktop", "quit_active"]
"ctrl+option+t" = ["spawn", "foot", "--title", "a b"]
"nonsense+" = ["desktop", "close_key"]
"cmd+w" = ["desktop", "no_such_command"]
"cmd+h" = []
"#,
    );
    let chord = |text| Chord::parse(text).expect("chord");
    assert_eq!(
        settings.bindings,
        Bindings(vec![
            Binding {
                chord: chord("cmd+q"),
                action: BindingAction::Desktop(DesktopCommand::QuitActive),
            },
            Binding {
                chord: chord("ctrl+option+t"),
                action: BindingAction::Spawn(Argv(
                    ["foot", "--title", "a b"].map(str::to_owned).to_vec()
                )),
            },
        ])
    );
}

/// Every kind of action survives the document: words, a file, a load, the model.
#[test]
fn every_kind_of_binding_survives_the_document() {
    let actions = [
        BindingAction::Spawn(Argv(vec![
            "foot".to_owned(),
            "-e".to_owned(),
            "htop".to_owned(),
        ])),
        BindingAction::Call(DbusCall {
            service: "org.quire.Shell1".to_owned(),
            path: "/org/quire/Shell1".to_owned(),
            interface: "org.quire.Shell1".to_owned(),
            method: "ToggleLauncher".to_owned(),
            args: vec!["a".to_owned(), "b".to_owned()],
        }),
        BindingAction::Call(DbusCall {
            service: "s".to_owned(),
            path: "/p".to_owned(),
            interface: "i".to_owned(),
            method: "m".to_owned(),
            args: Vec::new(),
        }),
        BindingAction::Desktop(DesktopCommand::CloseKey),
        BindingAction::Desktop(DesktopCommand::MinimiseKey),
        BindingAction::Desktop(DesktopCommand::HideActive),
        BindingAction::Desktop(DesktopCommand::HideOthers),
        BindingAction::Desktop(DesktopCommand::QuitActive),
        BindingAction::Desktop(DesktopCommand::ForceQuitPanel),
        BindingAction::Desktop(DesktopCommand::SwitchSpace(SpaceStep::Previous)),
        BindingAction::Desktop(DesktopCommand::SwitchSpace(SpaceStep::Next)),
        BindingAction::Desktop(DesktopCommand::SwitchSpace(SpaceStep::Number(3))),
        BindingAction::Desktop(DesktopCommand::Place(PlaceTarget::TopLeft)),
        BindingAction::Desktop(DesktopCommand::Place(PlaceTarget::Fill)),
        BindingAction::Desktop(DesktopCommand::Place(PlaceTarget::Restore)),
    ];
    let keys = "abcdefghijklmnopqrstuvwxyz".chars();
    let table: BTreeMap<String, ActionWords> = keys
        .zip(&actions)
        .map(|(key, action)| (format!("cmd+{key}"), ActionWords::of(action)))
        .collect();
    let doc = CasementDoc {
        bindings: BindingsSection(table),
        ..CasementDoc::default()
    };
    let (_dir, home) = home_with(&toml::to_string(&doc).expect("serialises"));
    let loaded = load(&home).expect("load");
    assert_eq!(loaded, doc);
    let mut got: Vec<BindingAction> = to_core(&loaded)
        .bindings
        .0
        .into_iter()
        .map(|binding| binding.action)
        .collect();
    let mut want = actions.to_vec();
    let order = |action: &BindingAction| format!("{action:?}");
    got.sort_by_key(order);
    want.sort_by_key(order);
    assert_eq!(got, want);
}

#[test]
fn the_schema_is_written_and_reads_back() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_schema(dir.path()).expect("schema");
    let text = std::fs::read_to_string(dir.path().join("casement.settings.toml")).expect("written");
    let read = ds_settings::schema::Schema::from_toml(&text).expect("parses");
    assert!(!read.key.is_empty(), "every settings key has a row");
    assert_eq!(read, schema());
    assert_eq!(read.app.0, "casement");
    assert_eq!(read.file.0, "casement/settings.toml");
}

/// A schema row for a key the document does not hold, or a document key with no row, is a
/// settings key a person could not set or one the Settings app would draw for nothing.
#[test]
fn the_schema_and_the_document_name_the_same_keys() {
    let doc = toml::Value::try_from(CasementDoc::default()).expect("serialises");
    let mut in_document = Vec::new();
    for (section, value) in doc.as_table().expect("a table") {
        if section == "bindings" {
            continue;
        }
        for (key, default) in value.as_table().expect("a section") {
            in_document.push((format!("{section}.{key}"), default.clone()));
        }
    }
    let in_schema: Vec<(String, toml::Value)> = schema()
        .key
        .into_iter()
        .map(|key| (key.path.0, key.default))
        .collect();
    let mut want = in_document;
    let mut got = in_schema;
    want.sort_by(|a, b| a.0.cmp(&b.0));
    got.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(got, want, "paths and defaults agree");
}

#[test]
fn the_binary_writes_the_schema_and_it_reads_back() {
    let dir = tempfile::tempdir().expect("temp dir");
    let status = Command::new(env!("CARGO_BIN_EXE_casement"))
        .arg("--write-schema")
        .arg(dir.path())
        .env_clear()
        .status()
        .expect("runs");
    assert!(status.success(), "--write-schema exits 0");
    let text = std::fs::read_to_string(dir.path().join("casement.settings.toml")).expect("written");
    assert_eq!(ds_settings::schema::Schema::from_toml(&text), Ok(schema()));
}
