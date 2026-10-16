//! The Privacy module over a private dbus-daemon, with every caller made to stand as a chosen
//! program (`BusPeers::introduce`): Settings reads and changes the list, the shell reads it,
//! nobody else touches it, and the administrator's rows cannot be switched. The model's side is
//! the channel the test reads. No test reads the real `/proc` for a caller.

use super::{PRIVACY_PATH, PrivacyEdit, PrivacyLink};
use crate::dbus::COMPOSITOR_BUS_NAME;
use crate::test_served::{Loop, Served, WHOLE_TEST, outcome, service};
use calloop::channel::Channel;
use casement_core::policy::Power;
use casement_core::privilege::{PrivacyRow, ProgramPath, Source, Subject, UnixSeconds, Verdict};
use ds_settings::live::LiveSchema;
use ds_settings::schema::{KeyKind, KeySpec};
use peephole_bus::peephole::fake::introduced_program;
use peephole_bus::peephole_core::Proof;
use zbus::zvariant::{OwnedValue, Value};

const NOT_PERMITTED: &str = "org.quire.SettingsModule1.Error.NotPermitted";
const ADMINISTRATOR: &str = "Set by your administrator";

fn program(path: &str) -> Subject {
    Subject::Program(ProgramPath::parse(path).expect("a path"))
}

fn row(path: &str, power: Power, verdict: Verdict, source: Source) -> PrivacyRow {
    PrivacyRow {
        power,
        subject: program(path),
        verdict,
        source,
        at: match source {
            Source::Person => Some(UnixSeconds(3)),
            Source::Administrator => None,
        },
    }
}

/// A served Compositor1 with the Privacy list on it: the person allowed `grim` the screen, an
/// administrator refused `evil` the screen. The channel is the model's end of Settings' changes.
fn served() -> (Served, Channel<PrivacyEdit>) {
    let (link, channel) = PrivacyLink::new();
    link.board().publish(vec![
        row(
            "/usr/bin/grim",
            Power::Screen,
            Verdict::Allow,
            Source::Person,
        ),
        row(
            "/usr/bin/evil",
            Power::Screen,
            Verdict::Deny,
            Source::Administrator,
        ),
    ]);
    (Served::start(WHOLE_TEST, Loop::Reads, Some(link)), channel)
}

async fn module(caller: &zbus::Proxy<'_>) -> zbus::Proxy<'static> {
    zbus::Proxy::new(
        caller.connection(),
        COMPOSITOR_BUS_NAME,
        PRIVACY_PATH,
        "org.quire.SettingsModule1",
    )
    .await
    .expect("proxy")
}

/// The module's schema, or the error's D-Bus name.
async fn describe(module: &zbus::Proxy<'_>) -> Result<LiveSchema, String> {
    match module.call::<_, _, String>("Describe", &()).await {
        Ok(json) => LiveSchema::from_json(&json).map_err(|error| error.to_string()),
        Err(zbus::Error::MethodError(name, ..)) => Err(name.to_string()),
        Err(other) => Err(other.to_string()),
    }
}

async fn set(module: &zbus::Proxy<'_>, key: &str, value: Value<'_>) -> String {
    outcome(module.call::<_, _, ()>("Set", &(key, value)).await)
}

async fn get(module: &zbus::Proxy<'_>, key: &str) -> String {
    let value: OwnedValue = module.call("Get", &(key,)).await.expect("a value");
    String::try_from(value).expect("a word")
}

fn key_of<'a>(schema: &'a LiveSchema, label: &str) -> &'a KeySpec {
    schema
        .key
        .iter()
        .find(|key| key.label.0 == label)
        .unwrap_or_else(|| panic!("no key labelled {label}"))
}

/// The list shows every power's section; a row the administrator set is a read-out that says so
/// and cannot be switched, and a refused switch reaches nobody.
#[tokio::test]
async fn the_administrators_rows_are_read_only() {
    let (served, channel) = served();
    let settings = served.caller(service("settings", Proof::Full)).await;
    let module = module(&settings).await;
    let schema = describe(&module).await.expect("Settings may read");

    let sections: std::collections::BTreeSet<&str> = schema
        .key
        .iter()
        .map(|key| key.section.0.as_str())
        .collect();
    assert_eq!(sections.len(), 7, "a section for each power");
    let evil = key_of(&schema, "/usr/bin/evil");
    assert!(matches!(evil.kind, KeyKind::Fixed { .. }));
    assert_eq!(
        evil.note.as_ref().map(|note| note.0.as_str()),
        Some(ADMINISTRATOR)
    );
    assert_eq!(get(&module, &evil.path.0).await, "off");
    let grim = key_of(&schema, "/usr/bin/grim");
    assert!(matches!(grim.kind, KeyKind::Toggle { .. }));
    assert_eq!(get(&module, &grim.path.0).await, "on");

    assert_eq!(
        set(&module, &evil.path.0, Value::from("on")).await,
        NOT_PERMITTED
    );
    assert!(channel.try_recv().is_err(), "nothing reached the model");
    assert_eq!(get(&module, &evil.path.0).await, "off");
}

/// Only the Settings app writes, proven all the way: the shell may read and not write, a program
/// no row names may not read, and Settings at less than full proof is refused too. Settings'
/// own change reaches the model's queue and reads back at once.
#[tokio::test]
async fn only_the_settings_app_may_write() {
    let (served, channel) = served();
    let settings = served.caller(service("settings", Proof::Full)).await;
    let schema = describe(&module(&settings).await).await.expect("read");
    let grim = key_of(&schema, "/usr/bin/grim").path.0.clone();
    let forget = format!("{grim}.forget");

    let shell = module(&served.caller(service("shell", Proof::Full)).await).await;
    assert!(describe(&shell).await.is_ok(), "the shell may read");
    assert_eq!(set(&shell, &grim, Value::from("off")).await, NOT_PERMITTED);

    let stranger = served
        .caller(introduced_program("/usr/bin/grim").expect("a standing"))
        .await;
    let stranger = module(&stranger).await;
    assert_eq!(
        describe(&stranger).await.map(|_| ()),
        Err(NOT_PERMITTED.to_owned())
    );
    assert_eq!(
        set(&stranger, &grim, Value::from("off")).await,
        NOT_PERMITTED
    );

    let half = module(&served.caller(service("settings", Proof::Exe)).await).await;
    assert_eq!(set(&half, &grim, Value::from("off")).await, NOT_PERMITTED);
    assert!(
        channel.try_recv().is_err(),
        "no refused write reached the model"
    );

    let module = module(&settings).await;
    assert_eq!(set(&module, &grim, Value::from("off")).await, "granted");
    assert_eq!(
        channel.try_recv().ok(),
        Some(PrivacyEdit::Deny {
            subject: program("/usr/bin/grim"),
            power: Power::Screen
        })
    );
    assert_eq!(
        get(&module, &grim).await,
        "off",
        "Settings reads its change back"
    );
    assert_eq!(set(&module, &forget, Value::from(true)).await, "granted");
    assert_eq!(
        channel.try_recv().ok(),
        Some(PrivacyEdit::Forget {
            subject: program("/usr/bin/grim"),
            power: Power::Screen
        })
    );
    assert_eq!(
        set(&module, &grim, Value::from(7_i64)).await,
        "org.quire.SettingsModule1.Error.BadValue"
    );
}
