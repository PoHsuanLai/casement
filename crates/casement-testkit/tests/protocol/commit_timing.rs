//! `commit_timing` (P0-H1).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A commit timed 50 ms ahead appears at the first vblank at or after it, on virtual time.
#[test]
#[ignore = "P0-H1"]
fn a_timestamped_commit_waits_for_its_instant() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H1: a commit timed 50 ms ahead appears at the first vblank at or after it, on virtual time"
    )
}
