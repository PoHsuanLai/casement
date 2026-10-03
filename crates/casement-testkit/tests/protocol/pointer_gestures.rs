//! `pointer_gestures` (P0-H4).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A two-finger swipe begins, updates and ends at the focused client.
#[test]
#[ignore = "P0-H4"]
fn a_swipe_the_system_does_not_take_reaches_the_client() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H4: a two-finger swipe begins, updates and ends at the focused client")
}
