//! `idle_notify` (P0-H8). The scenarios are written against the real `State`; they run once the
//! fixture and the test client core (P0-B-headless, P0-T-client) and the core loop's idle
//! hooks (P0-B-loop, interface asks of this lane) exist.

use casement_core::input::{DeviceClass, KeyCode};
use casement_core::policy::ConnectionRole;
use casement_core::time::Mono;
use casement_protocols::vocab::PressEdge;
use casement_testclient::misc::idle::{IdleKind, IdleLine, IdleSignal, IdleWatch};
use casement_testkit::{Fixture, FixtureSpec, TestOrigin};
use std::time::Duration;

/// Moves virtual time by `by` and reads what the watch received.
fn step(fixture: &mut Fixture, watch: &mut IdleWatch, by: Duration) {
    fixture.advance(by);
    watch.pump(fixture.model().now()).expect("pump the watch");
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
    fixture.input().key(keyboard, KeyCode(30), PressEdge::Press);
    fixture
        .input()
        .key(keyboard, KeyCode(30), PressEdge::Release);
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
    fixture.input().key(keyboard, KeyCode(30), PressEdge::Press);
    step(&mut fixture, &mut watch, Duration::from_secs(20));
    assert!(
        watch.lines().is_empty(),
        "30 s have not passed since the key"
    );
    step(&mut fixture, &mut watch, Duration::from_secs(10));
    assert_eq!(watch.signals(), vec![IdleSignal::Idled]);
}

/// CASW-07 (decided 2026-10-16): a peer nothing proves is served no less than 5 s, so a 1 ms
/// watch cannot read keystroke timing; the shell's own watch keeps the timeout it asked for.
#[test]
fn an_unproven_peer_asking_for_one_millisecond_is_told_after_five_seconds() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut watch = IdleWatch::start(
        fixture.client(client).globals(),
        IdleKind::InputOnly,
        Duration::from_millis(1),
    )
    .expect("the notification");
    fixture.pump();
    step(&mut fixture, &mut watch, Duration::from_millis(4_999));
    assert!(watch.lines().is_empty(), "nothing before the floor");
    step(&mut fixture, &mut watch, Duration::from_millis(1));
    assert_eq!(watch.signals(), vec![IdleSignal::Idled]);
}

/// NEW-CAS-2: a client may hold `MAX_IDLE_WATCHES` notifications; the next is over the cap and
/// the client is cut, so one client cannot make every input event walk a million watches.
#[test]
fn a_client_that_asks_for_idle_notifications_without_end_is_cut() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut made = Vec::new();
    for _ in 0..casement_wayland::limits::MAX_IDLE_WATCHES {
        made.push(
            IdleWatch::start(
                fixture.client(client).globals(),
                IdleKind::Inhibitable,
                Duration::from_secs(30),
            )
            .expect("a notification within the cap"),
        );
    }
    fixture.pump();
    assert!(
        fixture.client(client).is_connected(),
        "up to the cap is allowed"
    );
    made.push(
        IdleWatch::start(
            fixture.client(client).globals(),
            IdleKind::Inhibitable,
            Duration::from_secs(30),
        )
        .expect("the request is sent"),
    );
    fixture.pump();
    assert!(
        !fixture.client(client).is_connected(),
        "the client past the cap is cut"
    );
}
