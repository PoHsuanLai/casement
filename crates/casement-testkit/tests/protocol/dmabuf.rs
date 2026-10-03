//! `dmabuf` (P0-H6).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// With --dmabuf none there is no zwp_linux_dmabuf_v1 (shell-host F80 falls back to shm).
#[test]
#[ignore = "P0-H6"]
fn dmabuf_none_advertises_nothing() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H6: with --dmabuf none there is no zwp_linux_dmabuf_v1 (shell-host F80 falls back to shm)"
    )
}

/// With --dmabuf soft the default feedback names the fake main device.
#[test]
#[ignore = "P0-H6"]
fn dmabuf_soft_advertises_the_fake_main_device() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H6: with --dmabuf soft the default feedback names the fake main device")
}

/// A client dma-buf is tagged with its tranche's node (A5).
#[test]
#[ignore = "P0-H6"]
fn imported_buffers_are_tagged_with_their_node() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H6: a client dma-buf is tagged with its tranche's node (A5)")
}
