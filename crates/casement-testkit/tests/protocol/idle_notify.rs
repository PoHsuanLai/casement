//! `idle_notify` (P0-H8).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A 30 s notification fires idled after exactly 30 s of virtual time without input.
#[test]
#[ignore = "P0-H8"]
fn idle_is_notified_at_virtual_time() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H8: a 30 s notification fires idled after exactly 30 s of virtual time without input")
}

/// Input after idled sends resumed.
#[test]
#[ignore = "P0-H8"]
fn activity_resumes() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H8: input after idled sends resumed")
}
