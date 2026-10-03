//! `shm` (P0-H1).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Wl_shm advertises argb8888 and xrgb8888.
#[test]
#[ignore = "P0-H1"]
fn argb_and_xrgb_are_advertised() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H1: wl_shm advertises argb8888 and xrgb8888")
}

/// A buffer past its pool's end is invalid_stride/invalid_fd and disconnects only that client.
#[test]
#[ignore = "P0-H1"]
fn a_pool_too_small_for_its_buffer_is_a_protocol_error() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H1: a buffer past its pool's end is invalid_stride/invalid_fd and disconnects only that client"
    )
}
