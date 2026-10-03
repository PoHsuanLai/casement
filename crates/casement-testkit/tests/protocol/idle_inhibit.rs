//! `idle_inhibit` (P0-H8).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A visible surface with an inhibitor keeps idled from firing.
#[test]
#[ignore = "P0-H8"]
fn a_visible_inhibitor_holds_off_idle() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H8: a visible surface with an inhibitor keeps idled from firing")
}
