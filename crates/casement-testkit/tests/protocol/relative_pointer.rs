//! `relative_pointer` (P0-H4).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Relative_motion carries the device's raw delta.
#[test]
#[ignore = "P0-H4"]
fn relative_motion_is_unaccelerated() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H4: relative_motion carries the device's raw delta")
}
