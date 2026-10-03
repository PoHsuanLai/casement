//! `fifo` (P0-H1).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A second fifo commit is applied only after the first was presented.
#[test]
#[ignore = "P0-H1"]
fn a_fifo_barrier_waits_for_the_next_vblank() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H1: a second fifo commit is applied only after the first was presented")
}
