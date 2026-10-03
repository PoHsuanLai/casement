//! `xdg_toplevel_icon` (P0-H2).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A toplevel icon with a buffer is accepted without error.
#[test]
#[ignore = "P0-H2"]
fn an_icon_is_accepted() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H2: a toplevel icon with a buffer is accepted without error")
}
