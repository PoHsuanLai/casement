//! `session_lock` (P0-H3).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// While locked, keys and pointer reach only the lock surface.
#[test]
#[ignore = "P0-H3"]
fn lock_blocks_input_to_windows() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H3: while locked, keys and pointer reach only the lock surface")
}

/// Killing the locker keeps every output covered; nothing of the session is drawn.
#[test]
#[ignore = "P0-H3"]
fn a_dead_locker_leaves_the_lock_cover() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H3: killing the locker keeps every output covered; nothing of the session is drawn")
}

/// Locked arrives only after each output shows a lock surface or a cover.
#[test]
#[ignore = "P0-H3"]
fn locked_is_sent_once_every_output_is_covered() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H3: locked arrives only after each output shows a lock surface or a cover")
}
