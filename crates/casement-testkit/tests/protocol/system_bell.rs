//! `system_bell` (P0-H2).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Ring with a surface yields Effect::Bell for that window's app.
#[test]
#[ignore = "P0-H2"]
fn the_bell_rings_for_the_surfaces_app() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H2: ring with a surface yields Effect::Bell for that window's app")
}
