//! `idle_notify` (P0-H8). The scenarios are written against the real `State`; they run once the
//! fixture and the test client core (P0-B-headless, P0-T-client) and the core loop's idle
//! hooks (P0-B-loop, interface asks of this lane) exist.

use casement_core::input::{DeviceClass, Edge, KeyCode};
use casement_core::policy::ConnectionRole;
use casement_core::time::Mono;
use casement_testclient::misc::idle::{IdleKind, IdleLine, IdleSignal, IdleWatch};
use casement_testkit::{Fixture, FixtureSpec, TestOrigin};
use std::time::Duration;

/// Moves virtual time by `by` and reads what the watch received.
fn step(fixture: &mut Fixture, watch: &mut IdleWatch, by: Duration) {
    fixture.advance(by);
    watch.pump(fixture.model().now()).expect("pump the watch");
}

/// The notifier is advertised once, at version 2 (smithay's own global is withdrawn).
#[test]
fn the_notifier_is_advertised_once_at_version_2() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let notifiers: Vec<u32> = fixture
        .client(client)
        .globals()
        .contents()
        .with_list(|list| {
            list.iter()
                .filter(|global| global.interface == "ext_idle_notifier_v1")
                .map(|global| global.version)
                .collect()
        });
    assert_eq!(notifiers, vec![2]);
}

/// A 30 s notification fires idled after exactly 30 s of virtual time without input.
#[test]
fn idle_is_notified_at_virtual_time() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let asked_at = fixture.model().now();
    let mut watch = IdleWatch::start(
        fixture.client(client).globals(),
        IdleKind::Inhibitable,
        Duration::from_secs(30),
    )
    .expect("the notification");
    fixture.pump();
    step(&mut fixture, &mut watch, Duration::from_millis(29_999));
    assert!(watch.lines().is_empty(), "nothing a millisecond early");
    step(&mut fixture, &mut watch, Duration::from_millis(1));
    assert_eq!(
        watch.lines(),
        [IdleLine {
            at: Mono(asked_at.0 + Duration::from_secs(30)),
            signal: IdleSignal::Idled
        }]
    );
    step(&mut fixture, &mut watch, Duration::from_secs(60));
    assert_eq!(
        watch.signals(),
        vec![IdleSignal::Idled],
        "idled is sent once"
    );
}

/// Input after idled sends resumed, and idleness counts again from the input.
#[test]
fn activity_resumes() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let keyboard = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    fixture.pump();
    let mut watch = IdleWatch::start(
        fixture.client(client).globals(),
        IdleKind::Inhibitable,
        Duration::from_secs(30),
    )
    .expect("the notification");
    fixture.pump();
    step(&mut fixture, &mut watch, Duration::from_secs(30));
    assert_eq!(watch.signals(), vec![IdleSignal::Idled]);
    fixture.input().key(keyboard, KeyCode(30), Edge::Press);
    fixture.input().key(keyboard, KeyCode(30), Edge::Release);
    step(&mut fixture, &mut watch, Duration::ZERO);
    assert_eq!(
        watch.signals(),
        vec![IdleSignal::Idled, IdleSignal::Resumed]
    );
    step(&mut fixture, &mut watch, Duration::from_secs(30));
    assert_eq!(
        watch.signals(),
        vec![IdleSignal::Idled, IdleSignal::Resumed, IdleSignal::Idled]
    );
}

/// Activity before the timer is due moves the deadline: no idled at the original instant.
#[test]
fn input_moves_the_deadline() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let keyboard = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    fixture.pump();
    let mut watch = IdleWatch::start(
        fixture.client(client).globals(),
        IdleKind::Inhibitable,
        Duration::from_secs(30),
    )
    .expect("the notification");
    fixture.pump();
    step(&mut fixture, &mut watch, Duration::from_secs(10));
    fixture.input().key(keyboard, KeyCode(30), Edge::Press);
    step(&mut fixture, &mut watch, Duration::from_secs(20));
    assert!(
        watch.lines().is_empty(),
        "30 s have not passed since the key"
    );
    step(&mut fixture, &mut watch, Duration::from_secs(10));
    assert_eq!(watch.signals(), vec![IdleSignal::Idled]);
}
