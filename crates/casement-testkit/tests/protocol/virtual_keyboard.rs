//! `virtual_keyboard` (P0-H7).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A virtual keyboard's key enters the model as Origin::Virtual(Keyboard) and resets idleness (I8).
#[test]
#[ignore = "P0-H7"]
fn virtual_keys_are_origin_virtual_and_count_as_activity() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H7: a virtual keyboard's key enters the model as Origin::Virtual(Keyboard) and resets idleness (I8)"
    )
}
