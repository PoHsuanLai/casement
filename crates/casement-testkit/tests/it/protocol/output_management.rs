//! `output_management` (P1-M1): wlr-randr-style transcripts against virtual outputs: list the
//! heads, apply a mode, scale, position and transform, test a configuration, and the refused,
//! stale and malformed ones.

use super::consent::{ScriptedShell, kept};
use casement_core::OutputName;
use casement_core::geometry::PhysicalSize;
use casement_core::output::{Mode, ModePreference, OutputConfig, OutputPower};
use casement_core::policy::ConnectionRole;
use casement_core::privilege::{HELD_REQUESTS, Verdict};
use casement_core::time::Millihertz;
use casement_protocols::vocab::{Answer as Reply, Power};
use casement_testclient::manage::output_management::{
    Answer, ConfigId, HeadPlan, Intent, Lit, ModeChoice, OutputManager, Preference, Properties,
    Setting,
};
use casement_testkit::{ASKED_PROGRAM, Fixture, FixtureSpec};
use ds_core::geometry::scale::Scale;
use std::time::Duration;
use wayland_client::protocol::wl_output::Transform;

const TWO: [&str; 2] = [
    "HEADLESS-1:1920x1080@60000:120:0,0",
    "HEADLESS-2:2880x1620@60000:180:1920,0",
];

const SMALLER: Mode = Mode {
    size: PhysicalSize { w: 1280, h: 720 },
    refresh: Millihertz(60_000),
    preference: ModePreference::Other,
};

/// A fixture with `outputs`, and a manager that has read the heads.
pub(super) fn boot(outputs: &[&str]) -> (Fixture, casement_testkit::ClientId, OutputManager) {
    let mut fixture =
        Fixture::start(FixtureSpec::desktop().with_outputs(outputs)).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut manager = OutputManager::start(fixture.client(id).globals()).expect("the manager");
    settle(&mut fixture, &mut manager);
    (fixture, id, manager)
}

/// Lets the compositor and the manager's client talk until both are quiet.
pub(super) fn settle(fixture: &mut Fixture, manager: &mut OutputManager) {
    fixture.pump();
    manager.pump().expect("pump");
    fixture.pump();
    manager.pump().expect("pump");
}

pub(super) fn name(text: &str) -> OutputName {
    OutputName::parse(text).expect("a connector name")
}

pub(super) fn config_of(fixture: &Fixture, connector: &str) -> OutputConfig {
    fixture
        .model()
        .outputs()
        .0
        .values()
        .find(|state| state.facts.name == name(connector))
        .map(|state| state.config)
        .expect("a plugged output")
}

pub(super) fn plan(connector: &str, properties: Properties) -> HeadPlan {
    HeadPlan {
        name: connector.to_owned(),
        setting: Setting::Enable(properties),
    }
}

/// Binding the manager lists one head per output with its modes and state, then `done`.
#[test]
fn the_manager_lists_every_output_as_a_head() {
    let (fixture, id, manager) = boot(&TWO);
    let heads = manager.heads();
    assert_eq!(heads.len(), 2);
    assert!(manager.dones() >= 1);
    for head in heads {
        let config = config_of(&fixture, &head.name);
        let state = fixture
            .model()
            .outputs()
            .0
            .values()
            .find(|state| state.facts.name == name(&head.name))
            .expect("the head's output");
        assert_eq!(head.lit, Lit::On, "{}", head.name);
        assert_eq!(head.modes.len(), 1, "{}", head.name);
        let current = head.current.expect("a current mode");
        assert_eq!(
            (current.width, current.height, current.refresh),
            (
                i32::try_from(config.mode.size.w).expect("width"),
                i32::try_from(config.mode.size.h).expect("height"),
                60_000
            )
        );
        assert_eq!(current.preferred, Preference::Preferred);
        assert_eq!(head.position, (config.position.x, config.position.y));
        assert_eq!(head.transform, Transform::Normal);
        assert!(
            (head.scale - config.scale.as_f64()).abs() < 1e-9,
            "{}",
            head.name
        );
        assert_eq!(head.make.as_deref(), Some(state.facts.edid.make.as_str()));
        assert_eq!(head.model.as_deref(), Some(state.facts.edid.model.as_str()));
    }
    let mut fixture = fixture;
    assert!(fixture.client(id).protocol_error().is_none());
}

/// An applied configuration changes the mode, scale, position and transform; the model has them
/// and the head says so under a new serial.
#[test]
fn an_applied_configuration_changes_the_mode() {
    let (mut fixture, id, mut manager) = boot(&TWO[..1]);
    fixture.offer_modes(&name("HEADLESS-1"), &[SMALLER]);
    settle(&mut fixture, &mut manager);
    assert_eq!(
        manager.heads()[0].modes.len(),
        2,
        "the new mode is announced"
    );
    let before = manager.serial();

    let properties = Properties {
        mode: Some(ModeChoice::Offered {
            width: 1280,
            height: 720,
            refresh: 60_000,
        }),
        position: Some((100, 50)),
        transform: Some(Transform::_90),
        scale: Some(1.5),
    };
    let wanted = manager
        .configure(None, &[plan("HEADLESS-1", properties)], Intent::Apply)
        .expect("a configuration");
    settle(&mut fixture, &mut manager);

    assert_eq!(manager.answer(wanted), Some(Answer::Succeeded));
    let config = config_of(&fixture, "HEADLESS-1");
    assert_eq!(config.mode, SMALLER);
    assert_eq!(config.scale, Scale(180));
    assert_eq!((config.position.x, config.position.y), (100, 50));
    let head = &manager.heads()[0];
    let current = head.current.expect("a current mode");
    assert_eq!((current.width, current.height), (1280, 720));
    assert_eq!(head.position, (100, 50));
    assert_eq!(head.transform, Transform::_90);
    assert!((head.scale - 1.5).abs() < 1e-9);
    assert!(
        manager.serial() > before,
        "the heads moved under a new serial"
    );
    assert!(fixture.client(id).protocol_error().is_none());
}

/// Disabling a head darkens the output, enabling it lights it again; the other display stays lit.
#[test]
fn disabling_a_head_darkens_its_output() {
    let (mut fixture, _, mut manager) = boot(&TWO);
    let lit = |manager: &OutputManager, name: &str| {
        manager
            .heads()
            .iter()
            .find(|head| head.name == name)
            .map(|head| head.lit)
    };

    let off = HeadPlan {
        name: "HEADLESS-1".to_owned(),
        setting: Setting::Disable,
    };
    // A configuration names every head; the other one stays as it is.
    let other = plan("HEADLESS-2", Properties::default());
    let first = manager
        .configure(None, &[off, other.clone()], Intent::Apply)
        .expect("off");
    settle(&mut fixture, &mut manager);
    assert_eq!(manager.answer(first), Some(Answer::Succeeded));
    assert_eq!(lit(&manager, "HEADLESS-1"), Some(Lit::Off));
    assert_eq!(lit(&manager, "HEADLESS-2"), Some(Lit::On));
    assert_eq!(config_of(&fixture, "HEADLESS-1").power, OutputPower::Off);
    assert!(
        fixture.advance(Duration::from_millis(100)).frames > 0,
        "the other display draws"
    );

    let on = plan("HEADLESS-1", Properties::default());
    let second = manager
        .configure(None, &[on, other], Intent::Apply)
        .expect("on");
    settle(&mut fixture, &mut manager);
    assert_eq!(manager.answer(second), Some(Answer::Succeeded));
    assert_eq!(lit(&manager, "HEADLESS-1"), Some(Lit::On));
}

/// CASC-09: the only lit display cannot be disabled; the configuration fails and it keeps drawing.
#[test]
fn the_only_lit_head_cannot_be_disabled() {
    let (mut fixture, _, mut manager) = boot(&TWO[..1]);
    let off = HeadPlan {
        name: "HEADLESS-1".to_owned(),
        setting: Setting::Disable,
    };
    let refused = manager.configure(None, &[off], Intent::Apply).expect("off");
    settle(&mut fixture, &mut manager);
    assert_eq!(manager.answer(refused), Some(Answer::Failed));
    assert_eq!(manager.heads()[0].lit, Lit::On);
    assert!(fixture.advance(Duration::from_millis(100)).frames > 0);
}

/// A test changes nothing; a configuration built at an old serial is `cancelled`; a mode the
/// display does not offer, or a scale outside what outputs take, is `failed`.
#[test]
fn test_stale_and_refused_configurations_answer_correctly() {
    let (mut fixture, _, mut manager) = boot(&TWO[..1]);
    let before = config_of(&fixture, "HEADLESS-1");
    let serial = manager.serial();

    let doubled = Properties {
        scale: Some(2.0),
        ..Properties::default()
    };
    let tested = manager
        .configure(None, &[plan("HEADLESS-1", doubled)], Intent::Test)
        .expect("a test");
    settle(&mut fixture, &mut manager);
    assert_eq!(manager.answer(tested), Some(Answer::Succeeded));
    assert_eq!(
        config_of(&fixture, "HEADLESS-1"),
        before,
        "a test changes nothing"
    );
    assert_eq!(manager.serial(), serial, "and moves no serial");

    let stale = manager
        .configure(
            Some(serial.wrapping_sub(1)),
            &[plan("HEADLESS-1", doubled)],
            Intent::Apply,
        )
        .expect("a stale apply");
    let odd = Properties {
        mode: Some(ModeChoice::Custom {
            width: 1234,
            height: 567,
            refresh: 60_000,
        }),
        ..Properties::default()
    };
    let refused = manager
        .configure(None, &[plan("HEADLESS-1", odd)], Intent::Apply)
        .expect("a refused apply");
    let tiny = Properties {
        scale: Some(0.1),
        ..Properties::default()
    };
    let too_small = manager
        .configure(None, &[plan("HEADLESS-1", tiny)], Intent::Apply)
        .expect("a scale below the range");
    settle(&mut fixture, &mut manager);
    assert_eq!(manager.answer(stale), Some(Answer::Cancelled));
    assert_eq!(manager.answer(refused), Some(Answer::Failed));
    assert_eq!(manager.answer(too_small), Some(Answer::Failed));
    assert_eq!(config_of(&fixture, "HEADLESS-1"), before);
}

/// A configuration that leaves a head out is a protocol error (`unconfigured_head`).
#[test]
fn a_configuration_must_name_every_head() {
    let (mut fixture, id, mut manager) = boot(&TWO);
    manager
        .configure(
            None,
            &[plan("HEADLESS-1", Properties::default())],
            Intent::Apply,
        )
        .expect("a partial configuration");
    fixture.pump();
    let error = fixture.client(id).protocol_error().expect("an error");
    assert_eq!(error.interface, "zwlr_output_configuration_v1");
    assert_eq!(error.code, 2);
}

/// A head enabled twice in one configuration is a protocol error (`already_configured_head`).
#[test]
fn a_head_twice_in_one_configuration_is_an_error() {
    let (mut fixture, id, mut manager) = boot(&TWO[..1]);
    manager.enable_twice("HEADLESS-1").expect("the requests");
    fixture.pump();
    let error = fixture.client(id).protocol_error().expect("an error");
    assert_eq!(error.interface, "zwlr_output_configuration_v1");
    assert_eq!(error.code, 1);
}

/// A plugged output appears as a new head and an unplugged one is finished, each under a new
/// serial.
#[test]
fn hotplug_adds_and_finishes_heads() {
    let (mut fixture, _, mut manager) = boot(&TWO[..1]);
    let serial = manager.serial();
    fixture.plug(casement_backend::VirtualOutputSpec::parse(TWO[1]).expect("a spec"));
    settle(&mut fixture, &mut manager);
    assert_eq!(manager.heads().len(), 2);
    assert!(manager.serial() > serial);

    fixture.unplug(&name("HEADLESS-2"));
    settle(&mut fixture, &mut manager);
    assert_eq!(manager.heads().len(), 1);
    assert_eq!(manager.finished_heads(), 1);
}

/// A fixture with `outputs`, the scripted shell, and a manager held by a program that is asked
/// about its powers, which has read the heads.
fn asked_boot(outputs: &[&str]) -> (Fixture, ScriptedShell, OutputManager) {
    let mut fixture =
        Fixture::start(FixtureSpec::desktop().with_outputs(outputs)).expect("fixture");
    let shell = ScriptedShell::start(&mut fixture);
    let id = fixture.connect_via_ask();
    fixture.pump();
    let mut manager = OutputManager::start(fixture.client(id).globals()).expect("the manager");
    settle(&mut fixture, &mut manager);
    (fixture, shell, manager)
}

/// Reading the heads is free, but an `apply` or a `test` waits for the person: neither is
/// answered, nor does anything change, until they allow; then each is answered against the heads
/// as they are.
#[test]
fn a_configuration_waits_for_the_person_and_is_answered_on_allow() {
    let (mut fixture, shell, mut manager) = asked_boot(&TWO[..1]);
    assert_eq!(
        manager.heads().len(),
        1,
        "the heads were listed without asking"
    );
    assert!(shell.asked().is_empty());
    let before = config_of(&fixture, "HEADLESS-1");
    let moved = Properties {
        position: Some((100, 50)),
        ..Properties::default()
    };
    let tested = manager
        .configure(None, &[plan("HEADLESS-1", moved)], Intent::Test)
        .expect("a test");
    let applied = manager
        .configure(None, &[plan("HEADLESS-1", moved)], Intent::Apply)
        .expect("an apply");
    settle(&mut fixture, &mut manager);
    assert_eq!(shell.asked().len(), 1, "one question for both");
    assert_eq!(shell.asked()[0].power, Power::Displays);
    assert_eq!(manager.answer(tested), None);
    assert_eq!(manager.answer(applied), None);
    assert_eq!(config_of(&fixture, "HEADLESS-1"), before);

    shell.answer(&mut fixture, shell.only_ask(), Reply::Allow);
    settle(&mut fixture, &mut manager);
    assert_eq!(manager.answer(tested), Some(Answer::Succeeded));
    assert_eq!(manager.answer(applied), Some(Answer::Succeeded));
    let after = config_of(&fixture, "HEADLESS-1");
    assert_eq!((after.position.x, after.position.y), (100, 50));
}

/// A configuration that waited is checked against the heads when it is let through: one built
/// before the heads changed is `cancelled`.
#[test]
fn a_waiting_configuration_is_cancelled_when_the_heads_moved_meanwhile() {
    let (mut fixture, shell, mut manager) = asked_boot(&TWO[..1]);
    let applied = manager
        .configure(
            None,
            &[plan("HEADLESS-1", Properties::default())],
            Intent::Apply,
        )
        .expect("an apply");
    settle(&mut fixture, &mut manager);
    assert_eq!(manager.answer(applied), None);
    fixture.plug(casement_backend::VirtualOutputSpec::parse(TWO[1]).expect("a spec"));
    settle(&mut fixture, &mut manager);

    shell.answer(&mut fixture, shell.only_ask(), Reply::Allow);
    settle(&mut fixture, &mut manager);
    assert_eq!(manager.answer(applied), Some(Answer::Cancelled));
}

/// Don't Allow fails what waited, changes nothing, and is kept.
#[test]
fn a_waiting_configuration_fails_when_the_person_refuses() {
    let (mut fixture, shell, mut manager) = asked_boot(&TWO[..1]);
    let before = config_of(&fixture, "HEADLESS-1");
    let moved = Properties {
        position: Some((100, 50)),
        ..Properties::default()
    };
    let applied = manager
        .configure(None, &[plan("HEADLESS-1", moved)], Intent::Apply)
        .expect("an apply");
    settle(&mut fixture, &mut manager);
    shell.answer(&mut fixture, shell.only_ask(), Reply::Deny);
    settle(&mut fixture, &mut manager);
    assert_eq!(manager.answer(applied), Some(Answer::Failed));
    assert_eq!(config_of(&fixture, "HEADLESS-1"), before);
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Displays),
        Some(Verdict::Deny)
    );

    // Refused for good, the next one is failed at once without asking again.
    let again = manager
        .configure(None, &[plan("HEADLESS-1", moved)], Intent::Apply)
        .expect("another apply");
    settle(&mut fixture, &mut manager);
    assert_eq!(manager.answer(again), Some(Answer::Failed));
    assert_eq!(shell.asked().len(), 1);
}

/// A client keeps at most `HELD_REQUESTS` configurations waiting for the answer; the next one is
/// `failed` at once.
#[test]
fn a_client_holds_at_most_the_cap_of_waiting_configurations() {
    let (mut fixture, _shell, mut manager) = asked_boot(&TWO[..1]);
    let all: Vec<ConfigId> = (0..=HELD_REQUESTS)
        .map(|_| {
            manager
                .configure(
                    None,
                    &[plan("HEADLESS-1", Properties::default())],
                    Intent::Apply,
                )
                .expect("an apply")
        })
        .collect();
    settle(&mut fixture, &mut manager);
    let (kept_waiting, past) = all.split_at(HELD_REQUESTS);
    assert!(kept_waiting.iter().all(|id| manager.answer(*id).is_none()));
    assert_eq!(manager.answer(past[0]), Some(Answer::Failed));
}
