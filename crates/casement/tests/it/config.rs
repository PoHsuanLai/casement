//! The settings document (P0-F1-output): defaults when absent, one conversion into the model's
//! settings, and the schema detent reads (`casement --write-schema`). Every document lives in a
//! scratch directory standing in for `$XDG_CONFIG_HOME`; nothing here reads the real one.

use casement::cli::ConfigHome;
use casement::config::{
    ActionWords, BindingsSection, CasementDoc, Stamped, Version, load, schema, stamp_version,
    to_core, write_schema,
};
use casement_core::input::{
    Argv, Binding, BindingAction, Bindings, Chord, DbusCall, DesktopCommand, SpaceStep,
};
use casement_core::output::{Kelvin, ScalePolicy};
use casement_core::settings::{
    AccelProfile, CasementSettings, Enabled, FirstClick, MinimiseEffect, NightLightMode,
    RepeatRate, TitleDoubleClick,
};
use casement_core::window::SplitSide;
use casement_protocols::vocab::PlaceTarget;
use ds_core::geometry::scale::Scale;
use ds_core::vocab::Fraction;
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

/// Each row: where the settings come from, and that they are the defaults.
#[test]
fn the_defaults_come_from() {
    type Source = fn() -> CasementSettings;
    const CASES: &[(&str, Source)] = &[
        ("a missing document", || {
            let home = tempfile::tempdir().expect("temp dir");
            let doc = load(&ConfigHome::At(home.path().to_path_buf())).expect("load");
            assert_eq!(
                doc,
                CasementDoc::default().migrated(),
                "a missing document is the default one, at the current version"
            );
            to_core(&doc)
        }),
        ("a directory that does not exist", || {
            let home = ConfigHome::At(Path::new("/nonexistent/casement-test-home").to_path_buf());
            to_core(&load(&home).expect("load"))
        }),
        ("an empty document", || settings_of("")),
        ("the serialised default document", || {
            settings_of(&toml::to_string(&CasementDoc::default()).expect("serialises"))
        }),
    ];
    for (name, source) in CASES {
        assert_eq!(source(), CasementSettings::defaults(), "{name}");
    }
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
fn the_pointer_and_touchpad_keys_reach_the_model() {
    let settings = settings_of(
        r#"
[pointer]
accel = "flat"
natural_scroll = "on"
scroll_factor = 2000

[touchpad]
tap = "off"
disable_while_typing = "off"
"#,
    );
    let defaults = CasementSettings::defaults();
    assert_eq!(settings.pointer.mouse.accel, AccelProfile::Flat);
    assert_eq!(settings.pointer.mouse.natural_scroll, Enabled::On);
    assert_eq!(settings.pointer.mouse.scroll_factor, Fraction(2000));
    assert_eq!(settings.pointer.mouse.tap, defaults.pointer.mouse.tap);
    assert_eq!(settings.pointer.touchpad.tap, Enabled::Off);
    assert_eq!(settings.pointer.touchpad.disable_while_typing, Enabled::Off);
    assert_eq!(
        settings.pointer.touchpad.natural_scroll,
        defaults.pointer.touchpad.natural_scroll
    );
}

#[test]
fn the_gesture_keys_reach_the_model() {
    use casement_core::settings::SwipeFingers;
    let settings = settings_of(
        r#"
[gestures]
spaces = "three"
natural = "off"
"#,
    );
    let defaults = CasementSettings::defaults();
    assert_eq!(settings.gestures.spaces, SwipeFingers::Three);
    assert_eq!(settings.gestures.natural, Enabled::Off);
    assert_eq!(
        settings.gestures.mission_control,
        defaults.gestures.mission_control
    );
}

#[test]
fn the_rendering_gpu_is_auto_or_a_pci_address() {
    use casement_core::gpu::{ByPathId, GpuChoice};
    let render = |text: &str| settings_of(text).gpu.render;
    assert_eq!(render(""), GpuChoice::Auto);
    assert_eq!(render("[gpu]\nrender = \"auto\"\n"), GpuChoice::Auto);
    assert_eq!(
        render("[gpu]\nrender = \"0000:01:00.0\"\n"),
        GpuChoice::ByPath(ByPathId::parse("0000:01:00.0").expect("id"))
    );
    assert_eq!(
        render("[gpu]\nrender = \"the big one\"\n"),
        GpuChoice::Auto,
        "text that is neither costs only itself"
    );
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

/// Each row: what the document says about `keyboard.repeat_rate`, and the rate the model gets.
#[test]
fn the_repeat_rate_follows_the_document() {
    let cases = [
        (
            "a rate that would never fire is one a second",
            0,
            RepeatRate(1),
        ),
        ("a rate is kept", 30, RepeatRate(30)),
    ];
    for (name, rate, want) in cases {
        let settings = settings_of(&format!("[keyboard]\nrepeat_rate = {rate}\n"));
        assert_eq!(settings.keyboard.repeat_rate, want, "{name}");
    }
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
        BindingAction::Desktop(DesktopCommand::MissionControl),
        BindingAction::Desktop(DesktopCommand::SwitchSpace(SpaceStep::Previous)),
        BindingAction::Desktop(DesktopCommand::SwitchSpace(SpaceStep::Next)),
        BindingAction::Desktop(DesktopCommand::SwitchSpace(SpaceStep::Number(3))),
        BindingAction::Desktop(DesktopCommand::Place(PlaceTarget::TopLeft)),
        BindingAction::Desktop(DesktopCommand::Place(PlaceTarget::Fill)),
        BindingAction::Desktop(DesktopCommand::Place(PlaceTarget::Restore)),
        BindingAction::Desktop(DesktopCommand::SplitTile(SplitSide::Left)),
        BindingAction::Desktop(DesktopCommand::SplitTile(SplitSide::Right)),
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
    assert_eq!(loaded, doc.migrated());
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

/// The schema is written through the library and through the binary; both read back as
/// `schema()`.
#[test]
fn the_schema_is_written_and_reads_back() {
    let read_back = |dir: &Path, step: &str| {
        let text = std::fs::read_to_string(dir.join("casement.settings.toml"))
            .unwrap_or_else(|e| panic!("{step}: written: {e}"));
        ds_settings::schema::Schema::from_toml(&text)
            .unwrap_or_else(|e| panic!("{step}: parses: {e:?}"))
    };
    let dir = tempfile::tempdir().expect("temp dir");
    write_schema(dir.path()).expect("schema");
    let read = read_back(dir.path(), "write_schema");
    assert!(!read.key.is_empty(), "every settings key has a row");
    assert_eq!(read, schema(), "write_schema");
    assert_eq!(read.app.0, "casement");
    assert_eq!(read.file.0, "casement/settings.toml");

    let dir = tempfile::tempdir().expect("temp dir");
    let status = Command::new(env!("CARGO_BIN_EXE_casement"))
        .arg("--write-schema")
        .arg(dir.path())
        .env_clear()
        .status()
        .expect("runs");
    assert!(status.success(), "--write-schema exits 0");
    assert_eq!(read_back(dir.path(), "the binary"), schema(), "the binary");
}

/// The binary answers `--help` (a smoke test beside the other one that runs it).
#[test]
fn the_binary_answers_help() {
    let status = Command::new(env!("CARGO_BIN_EXE_casement"))
        .arg("--help")
        .env_clear()
        .status()
        .expect("the casement binary runs");
    assert!(status.success());
}

/// Each row: a file as an older casement wrote it, and what a mouse's natural scrolling is when
/// it is read. The old default (off) was written by every save; only a file that predates the
/// version loses it, and a value the person set to anything else, or a file that says it is
/// current, is kept.
#[test]
fn an_old_file_drops_the_retired_default_and_keeps_what_the_person_set() {
    let table: &[(&str, &str, Enabled)] = &[
        ("no file keys", "", Enabled::On),
        (
            "the old default, no version",
            "[pointer]\nnatural_scroll = \"off\"\n",
            Enabled::On,
        ),
        (
            "the old default, version 1",
            "version = 1\n[pointer]\nnatural_scroll = \"off\"\n",
            Enabled::On,
        ),
        (
            "on, no version",
            "[pointer]\nnatural_scroll = \"on\"\n",
            Enabled::On,
        ),
        (
            "off, chosen after the migration",
            "version = 2\n[pointer]\nnatural_scroll = \"off\"\n",
            Enabled::Off,
        ),
        (
            "off in a file from a newer casement",
            "version = 9\n[pointer]\nnatural_scroll = \"off\"\n",
            Enabled::Off,
        ),
    ];
    for (what, text, want) in table {
        assert_eq!(
            settings_of(text).pointer.mouse.natural_scroll,
            *want,
            "{what}"
        );
    }
}

/// The other keys of an old file are the person's, whatever they equal.
#[test]
fn migration_touches_only_the_retired_default() {
    let settings = settings_of(
        "[pointer]\nnatural_scroll = \"off\"\ntap = \"on\"\n[touchpad]\nnatural_scroll = \"off\"\n",
    );
    assert_eq!(settings.pointer.mouse.natural_scroll, Enabled::On);
    assert_eq!(settings.pointer.mouse.tap, Enabled::On);
    assert_eq!(
        settings.pointer.touchpad.natural_scroll,
        Enabled::Off,
        "the touchpad's default never changed, so this was a choice"
    );
}

#[test]
fn a_migrated_document_is_written_at_the_current_version() {
    let doc = CasementDoc::default().migrated();
    assert_eq!(doc.version, Some(Version::CURRENT));
    let text = toml::to_string(&doc).expect("serialises");
    assert!(text.starts_with("version = 2\n"), "{text}");
    let (_dir, home) = home_with(&text);
    assert_eq!(load(&home).expect("load"), doc);
}

#[test]
fn stamping_writes_only_the_version_and_only_once() {
    let old = "# mine\n[pointer]\nnatural_scroll = \"off\"\n";
    let (dir, home) = home_with(old);
    let file = dir.path().join("casement/settings.toml");
    assert_eq!(stamp_version(&home).expect("stamp"), Stamped::Written);
    let stamped = std::fs::read_to_string(&file).expect("read");
    assert_eq!(stamped, format!("version = 2\n{old}"));
    assert_eq!(stamp_version(&home).expect("stamp"), Stamped::Left);
    assert_eq!(std::fs::read_to_string(&file).expect("read"), stamped);
    // The choice made after the stamp survives the next read.
    assert_eq!(
        to_core(&load(&home).expect("load"))
            .pointer
            .mouse
            .natural_scroll,
        Enabled::Off
    );
}

#[test]
fn stamping_a_missing_file_writes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let home = ConfigHome::At(dir.path().to_path_buf());
    assert_eq!(stamp_version(&home).expect("stamp"), Stamped::Left);
    assert!(!dir.path().join("casement").exists());
}

/// Each row: what a change in the file does to the settings casement runs with. Only the
/// pointer, the key repeat, the keyboard layout and the night light follow (sill rewrites the
/// night light's keys when Night Shift is switched); the bindings the keymap gave stay.
#[test]
fn the_file_changes_the_pointer_the_repeat_the_layout_and_the_night_light_and_nothing_else() {
    use casement::settings_live::followed;
    let current = CasementSettings::defaults();
    let doc = |text: &str| -> CasementDoc {
        let (_dir, home) = home_with(text);
        load(&home).expect("load")
    };
    let same = followed(&current, &doc(""));
    assert_eq!(
        same, current,
        "an empty file is the defaults, which it already has"
    );

    let next = followed(
        &current,
        &doc(
            "[pointer]\ntap = \"on\"\n[keyboard]\nrepeat_rate = 30\nxkb_layout = \"fr\"\n[windows]\nquit_grace_ms = 9000\n[night_light]\nmode = \"always\"\nkelvin = 2900\nskip_outputs = [\"DP-1\", \"edid:ACME/M1/S1\"]\n",
        ),
    );
    assert_eq!(
        next.night_light.mode,
        NightLightMode::Always,
        "night light follows"
    );
    assert_eq!(next.night_light.warmth, Kelvin(2900));
    assert_eq!(next.night_light.skip_outputs, ["DP-1", "edid:ACME/M1/S1"]);
    assert_eq!(
        followed(&next, &doc("")).night_light,
        CasementSettings::defaults().night_light,
        "a file without the section is the defaults again, the skip list included"
    );
    assert_eq!(next.pointer.mouse.tap, Enabled::On);
    assert_eq!(next.keyboard.repeat_rate, RepeatRate(30));
    assert_eq!(next.keyboard.layout.layout, "fr", "the layout follows");
    assert_eq!(
        followed(&next, &doc("[keyboard]\nxkb_layout = \"fr\"\n"))
            .keyboard
            .layout,
        next.keyboard.layout,
        "the same layout again is no change"
    );
    assert_eq!(next.windows, current.windows);
    assert_eq!(next.bindings, current.bindings);
}
