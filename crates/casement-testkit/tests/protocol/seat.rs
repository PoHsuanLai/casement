//! `seat` (P0-H4).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A wheel detent arrives as axis_source(wheel), axis_value120(120), axis, frame (SP1 B1).
#[test]
#[ignore = "P0-H4"]
fn axis_source_and_value120_are_sent() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H4: a wheel detent arrives as axis_source(wheel), axis_value120(120), axis, frame (SP1 B1)"
    )
}

/// Repeat_info carries the keyboard settings' rate and delay.
#[test]
#[ignore = "P0-H4"]
fn repeat_info_comes_from_settings() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H4: repeat_info carries the keyboard settings' rate and delay")
}

/// Enter, modifiers, key: the order matches tests/golden/transcripts/seat-modifiers.txt.
#[test]
#[ignore = "P0-H4"]
fn modifiers_ordering_transcript() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H4: enter, modifiers, key: the order matches tests/golden/transcripts/seat-modifiers.txt"
    )
}

/// The surface now under a resting pointer receives enter at once (A16, SP1 A1).
#[test]
#[ignore = "P0-H4"]
fn pointer_focus_is_recomputed_when_the_surface_under_it_unmaps() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H4: the surface now under a resting pointer receives enter at once (A16, SP1 A1)")
}
