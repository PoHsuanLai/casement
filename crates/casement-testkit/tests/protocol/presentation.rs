//! `presentation` (P0-H1).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Presented carries the virtual vblank instant and sequence; discarded when the surface is covered.
#[test]
#[ignore = "P0-H1"]
fn feedback_arrives_on_virtual_time() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H1: presented carries the virtual vblank instant and sequence; discarded when the surface is covered"
    )
}
