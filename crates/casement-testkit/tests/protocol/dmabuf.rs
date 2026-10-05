//! `dmabuf` (P0-H6).
//!
//! Written against the API that exists: the fixture's clients expose `globals()`. The steps
//! that need more (a dma-buf mode on `FixtureSpec`, a client's `Connection` to run the
//! feedback probe, udmabuf client buffers) are `todo!` with the missing piece named; every
//! test stays ignored until the fixture itself runs (`Fixture::start` is P0-B-headless's).

use casement_backend::DmabufMode;
use casement_core::policy::ConnectionRole;
use casement_testclient::misc::dmabuf_feedback::{FeedbackProbe, TableRow, advertised_version};
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
fn dmabuf_soft_advertises_the_fake_main_device() {
    let mut fixture =
        Fixture::start(FixtureSpec::desktop().with_dmabuf(DmabufMode::Soft)).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    assert_eq!(
        advertised_version(fixture.client(client).globals()),
        Some(6)
    );
    let connection = fixture.client(client).connection().clone();
    let mut probe = FeedbackProbe::start(&connection, fixture.client(client).globals())
        .expect("the feedback probe");
    fixture.pump();
    let report = probe.report().expect("the feedback said done");
    assert_eq!(
        report.device(),
        Some(0xE280),
        "226:128, the first fabricated node"
    );
    let first = report.tranches.first().expect("a tranche");
    assert_eq!(first.target_device, Some(0xE280));
    let linear = |format: u32| TableRow {
        format,
        modifier: 0,
    };
    // ARGB8888 and XRGB8888 are the fourcc codes 'AR24' and 'XR24'; linear is modifier 0.
    assert_eq!(
        first.formats,
        [linear(0x3432_5241), linear(0x3432_5258)],
        "linear ARGB8888 and XRGB8888"
    );
}

/// A client dma-buf is tagged with its tranche's node (A5).
#[test]
#[ignore = "body is todo!(): needs a udmabuf client buffer through the testclient and the compositor's view of the imported Dmabuf (copy counters wait on P1-D4)"]
fn imported_buffers_are_tagged_with_their_node() {
    let mut fixture =
        Fixture::start(FixtureSpec::desktop().with_dmabuf(DmabufMode::Soft)).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H6: create_immed a linear ARGB8888 udmabuf; the compositor's Dmabuf::node() is the main device's render node"
    )
}
