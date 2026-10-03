//! `compositor` (P0-H1).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Wl_compositor binds at versions 1 to 5; a surface with a buffer commits; the transcript matches tests/golden/transcripts/compositor-commit.txt.
#[test]
#[ignore = "P0-H1"]
fn every_version_binds_and_a_surface_commits() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H1: wl_compositor binds at versions 1 to 5; a surface with a buffer commits; the transcript matches tests/golden/transcripts/compositor-commit.txt"
    )
}

/// A commit on a surface with no role yet changes nothing in the model.
#[test]
#[ignore = "P0-H1"]
fn a_commit_without_a_role_is_unassigned() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H1: a commit on a surface with no role yet changes nothing in the model")
}

/// Destroying a surface before its buffer, and the reverse, leave no texture behind.
#[test]
#[ignore = "P0-H1"]
fn destroying_a_surface_before_its_buffer_is_clean() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H1: destroying a surface before its buffer, and the reverse, leave no texture behind")
}
