//! `dmabuf` (P0-H6).
//!
//! Written against the API that exists: the fixture's clients expose `globals()`. The steps
//! that need more (a dma-buf mode on `FixtureSpec`, a client's `Connection` to run the
//! feedback probe, udmabuf client buffers) are `todo!` with the missing piece named; every
//! test stays ignored until the fixture itself runs (`Fixture::start` is P0-B-headless's).

use casement_core::policy::ConnectionRole;
use casement_testclient::misc::dmabuf_feedback::advertised_version;
use casement_testkit::{Fixture, FixtureSpec};

/// With --dmabuf none there is no zwp_linux_dmabuf_v1 (shell-host F80 falls back to shm).
#[test]
fn dmabuf_none_advertises_nothing() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    assert_eq!(advertised_version(fixture.client(client).globals()), None);
}

/// With --dmabuf soft the default feedback names the fake main device (version 6, A3).
#[test]
#[ignore = "body is todo!(): FixtureSpec has no with_dmabuf(DmabufMode::Soft) yet, so the fixture cannot start in soft mode"]
fn dmabuf_soft_advertises_the_fake_main_device() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _ = fixture.client(client);
    // Once both exist: advertised_version(globals) == Some(6); a FeedbackProbe::start(
    // client.connection(), client.globals()) reports device() == Some(0xE280) (226:128) and
    // a first tranche of linear ARGB8888 and XRGB8888 targeting it.
    todo!("P0-H6: soft advertises version 6 and a feedback whose main device is 226:128")
}

/// A client dma-buf is tagged with its tranche's node (A5).
#[test]
#[ignore = "body is todo!(): needs a soft-dmabuf fixture (FixtureSpec::with_dmabuf) and a udmabuf client buffer; copy counters wait on P1-D4"]
fn imported_buffers_are_tagged_with_their_node() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H6: create_immed a linear ARGB8888 udmabuf; the compositor's Dmabuf::node() is the main device's render node"
    )
}
