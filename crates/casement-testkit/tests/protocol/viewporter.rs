//! `viewporter` (P0-H1).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A viewport's source and destination reach the frame (pixman golden).
#[test]
#[ignore = "P0-H1"]
fn a_viewport_crops_and_scales() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H1: a viewport's source and destination reach the frame (pixman golden)")
}
