//! `xdg_toplevel_tag` (P0-H2).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Set_toplevel_tag is kept for placement memory.
#[test]
#[ignore = "P0-H2"]
fn a_tag_is_stored() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H2: set_toplevel_tag is kept for placement memory")
}
