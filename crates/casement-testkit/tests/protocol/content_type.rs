//! `content_type` (P0-H1).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Set_content_type(video) is read at commit.
#[test]
#[ignore = "P0-H1"]
fn the_content_type_is_stored() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H1: set_content_type(video) is read at commit")
}
