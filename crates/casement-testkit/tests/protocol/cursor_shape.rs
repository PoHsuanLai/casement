//! `cursor_shape` (P0-H4).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Set_shape(text) with the pointer's enter serial sets the named cursor.
#[test]
#[ignore = "body is todo!(): the P0-H4 stub was never written (the H4 patch has no cursor-shape step)"]
fn a_shape_request_sets_the_named_cursor() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H4: set_shape(text) with the pointer's enter serial sets the named cursor")
}
