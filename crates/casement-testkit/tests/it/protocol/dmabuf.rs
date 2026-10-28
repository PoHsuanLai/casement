//! `dmabuf` (P0-H6).

use casement_core::geometry::PhysicalSize;
use casement_core::policy::ConnectionRole;
use casement_testclient::Pattern;
use casement_testclient::misc::dmabuf_feedback::{FeedbackProbe, TableRow, advertised_version};
use casement_testclient::udmabuf::UdmabufSource;
use casement_testkit::udmabuf::client_buffer;
use casement_testkit::{Fixture, FixtureSpec};
use smithay::reexports::drm::node::NodeType;

/// With --dmabuf soft the default feedback names the fake main device (version 6, A3).
#[test]
fn dmabuf_soft_advertises_the_fake_main_device() {
    let mut fixture = Fixture::start(FixtureSpec::desktop().with_dmabuf(true)).expect("fixture");
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
fn imported_buffers_are_tagged_with_their_node() {
    let mut fixture = Fixture::start(FixtureSpec::desktop().with_dmabuf(true)).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    // A 64x64 linear ARGB8888 buffer: its stride (256) is what /dev/udmabuf accepts.
    let size = PhysicalSize { w: 64, h: 64 };
    let source = UdmabufSource::new(size, Pattern::Solid(0xff33_6699)).expect("a source");
    let buffer = client_buffer(fixture.client(client), &source).expect("a dma-buf buffer");
    fixture.pump();
    assert!(
        fixture.client(client).protocol_error().is_none(),
        "create_immed of an offered format is not an error"
    );
    let imported = fixture
        .imported_dmabuf(client, &buffer)
        .expect("the compositor holds the import");
    // `DrmNode::from_dev_id` types a device by reading /sys/dev/char/<major>:<minor>/device/drm,
    // which only exists in a jail run with `--fabricate-gpus` (dev/gate.sh does; a bare jail
    // does not). Without it nothing can say what the main device is, and the buffer is left
    // untagged: smithay then skips its cross-GPU copy rather than guessing. On a host with a
    // real GPU the same read succeeds and the buffer carries the real render node; what must
    // never happen is an untyped or non-render node.
    match fabricated_gpus() {
        0 => {
            if let Some(node) = imported.node() {
                assert_eq!(node.ty(), NodeType::Render, "a real node is a render node");
            }
        }
        _ => {
            let node = imported.node().expect("the import is tagged with a node");
            assert_eq!(node.ty(), NodeType::Render, "the main device's render node");
            assert_eq!(
                node.dev_id(),
                0xE280,
                "226:128, the node the default feedback names"
            );
        }
    }
}

/// How many soft GPU nodes the jail fabricated (`JAIL_GPUS`; none outside a jail run with the
/// flag).
fn fabricated_gpus() -> u32 {
    std::env::var("JAIL_GPUS")
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(0)
}
