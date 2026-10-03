//! `shortcuts_inhibit` (P0-H4).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// With an active inhibitor a binding chord reaches the client instead of firing.
#[test]
#[ignore = "P0-H4"]
fn an_active_inhibitor_receives_binding_chords() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H4: with an active inhibitor a binding chord reaches the client instead of firing")
}
