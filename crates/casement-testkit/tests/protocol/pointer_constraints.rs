//! `pointer_constraints` (P0-H4).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A locked pointer stays put and relative motion still arrives.
#[test]
#[ignore = "P0-H4"]
fn a_lock_holds_while_focused() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H4: a locked pointer stays put and relative motion still arrives")
}
