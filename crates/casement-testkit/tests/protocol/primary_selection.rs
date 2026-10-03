//! `primary_selection` (P0-H5).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// The primary selection is offered to the focused client.
#[test]
#[ignore = "P0-H5"]
fn primary_selection_follows_focus() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H5: the primary selection is offered to the focused client")
}
