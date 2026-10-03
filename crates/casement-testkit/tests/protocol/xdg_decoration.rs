//! `xdg_decoration` (P0-H2).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A client asking for server-side decoration is told client_side at P0.
#[test]
#[ignore = "P0-H2"]
fn client_side_is_preferred() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H2: a client asking for server-side decoration is told client_side at P0")
}
