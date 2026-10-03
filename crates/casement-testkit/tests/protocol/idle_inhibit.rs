//! `idle_inhibit` (P0-H8). Runs once the fixture and test client core land.

use casement_core::policy::ConnectionRole;
use casement_testclient::misc::idle::{IdleKind, IdleSignal, IdleWatch};
use casement_testclient::misc::idle_inhibit::InhibitHold;
use casement_testkit::{Fixture, FixtureSpec};
use std::time::Duration;

/// Moves virtual time by `by` and reads what the watch received.
fn step(fixture: &mut Fixture, watch: &mut IdleWatch, by: Duration) {
    fixture.advance(by);
    watch.pump(fixture.model().now()).expect("pump the watch");
}

/// An inhibited surface keeps idled from firing; releasing it restarts the count from the
/// release. (The surface here is alive but has no window, so it is the alive-surface rule of
/// `IdleInhibitPart::inhibited` that holds idleness off; shown-Space visibility waits for the
/// window lookup.)
#[test]
#[ignore = "P0-H8: written; runs once P0-B-headless (Fixture::start) and P0-T-client land"]
fn a_visible_inhibitor_holds_off_idle() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut hold = InhibitHold::start(fixture.client(client).globals()).expect("the inhibitor");
    let mut watch = IdleWatch::start(
        fixture.client(client).globals(),
        IdleKind::Inhibitable,
        Duration::from_secs(30),
    )
    .expect("the notification");
    fixture.pump();
    step(&mut fixture, &mut watch, Duration::from_secs(120));
    assert!(watch.lines().is_empty(), "an inhibitor holds idled off");
    hold.pump().expect("pump the hold");
    hold.release().expect("release");
    fixture.pump();
    step(&mut fixture, &mut watch, Duration::from_millis(29_999));
    assert!(
        watch.lines().is_empty(),
        "the count restarts at the release"
    );
    step(&mut fixture, &mut watch, Duration::from_millis(1));
    assert_eq!(watch.signals(), vec![IdleSignal::Idled]);
}

/// A `get_input_idle_notification` counts input only: an inhibitor does not hold it off.
#[test]
#[ignore = "P0-H8: written; runs once P0-B-headless (Fixture::start) and P0-T-client land"]
fn an_input_idle_notification_ignores_inhibitors() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _hold = InhibitHold::start(fixture.client(client).globals()).expect("the inhibitor");
    let mut watch = IdleWatch::start(
        fixture.client(client).globals(),
        IdleKind::InputOnly,
        Duration::from_secs(30),
    )
    .expect("the notification");
    fixture.pump();
    step(&mut fixture, &mut watch, Duration::from_secs(30));
    assert_eq!(watch.signals(), vec![IdleSignal::Idled]);
}
