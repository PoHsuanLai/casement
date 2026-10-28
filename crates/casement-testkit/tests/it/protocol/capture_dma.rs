//! Capture into dma-bufs (wave-2026-10-15/capture3): a session offers the dma-buf formats the
//! renderer can draw a frame into, next to shared memory, and `zwlr_screencopy` says
//! `linux_dmabuf`. The offer is the renderer's set (pixman: linear ARGB and XRGB) among the
//! formats the dma-buf global accepts, so a capture session needs `--dmabuf soft` to offer any.
//!
//! Drawing a frame into a client's dma-buf is the renderer's (`casement_render::pass::
//! write_dmabuf`, a real-run check); these tests read the offer from the client's side.

use crate::protocol::image_capture::{name, opened, small};
use casement_core::policy::ConnectionRole;
use casement_testclient::Pumped;
use casement_testclient::capture::{CopyArea, CursorMode, ScreenCopy};
use casement_testkit::{Fixture, FrameCount};

/// ARGB8888 and XRGB8888 as fourcc codes ('AR24', 'XR24').
const ARGB: u32 = 0x3432_5241;
const XRGB: u32 = 0x3432_5258;

/// How many soft GPU nodes the jail fabricated (`JAIL_GPUS`; none outside a jail run with the
/// flag). Without one the main device cannot be typed as a render node, and an ext session
/// has nobody to name as the device to allocate on.
fn fabricated_gpus() -> u32 {
    std::env::var("JAIL_GPUS")
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(0)
}

/// Pumps the compositor and the copy until neither moves.
fn settle_copy(fixture: &mut Fixture, copy: &mut ScreenCopy) {
    for _ in 0..100 {
        let compositor = fixture.pump();
        let side = copy.pump().expect("pump the copy");
        if (compositor, side) == (Pumped::Quiet, Pumped::Quiet) {
            return;
        }
    }
    panic!("the compositor and the copy did not go quiet");
}

/// With `--dmabuf soft` an output session offers linear ARGB8888 and XRGB8888 and the main
/// device (226:128), once the jail has a node to type it by.
#[test]
fn a_session_offers_dma_buf_where_the_renderer_draws_into_it() {
    let (_fixture, _client, capture) = opened(small().with_dmabuf(true), "HEADLESS-1");
    let formats = capture.dmabuf_formats();
    match fabricated_gpus() {
        0 => {
            if !formats.is_empty() {
                // A host with a real GPU types the device itself.
                assert_eq!(capture.dmabuf_device(), Some(0xE280));
            }
        }
        _ => {
            assert_eq!(capture.dmabuf_device(), Some(0xE280), "the main device");
            let linear = |code: u32| {
                formats
                    .iter()
                    .any(|f| f.format == code && f.modifiers == [0])
            };
            assert!(linear(ARGB), "linear ARGB8888 in {formats:?}");
            assert!(linear(XRGB), "linear XRGB8888 in {formats:?}");
        }
    }
    let session = capture.session().expect("a session");
    assert!(
        !session.formats.is_empty(),
        "shared memory stays on offer beside dma-buf"
    );
}

/// Without a dma-buf global nothing can be allocated, so only shared memory is offered.
#[test]
fn a_session_without_a_dma_buf_global_offers_shared_memory_only() {
    let (_fixture, _client, capture) = opened(small(), "HEADLESS-1");
    assert!(capture.dmabuf_formats().is_empty());
    assert_eq!(capture.dmabuf_device(), None);
    assert!(!capture.session().expect("a session").formats.is_empty());
}

/// `zwlr_screencopy_frame_v1.linux_dmabuf` names the same formats at the frame's size.
#[test]
fn a_wlr_frame_announces_linux_dmabuf() {
    let mut fixture = Fixture::start(small().with_dmabuf(true)).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut copy = ScreenCopy::start(fixture.client(client).globals()).expect("the manager");
    settle_copy(&mut fixture, &mut copy);
    copy.capture("HEADLESS-1", CopyArea::Output, CursorMode::Hidden)
        .expect("a frame");
    settle_copy(&mut fixture, &mut copy);
    let mut offered: Vec<u32> = copy
        .dmabuf_offers()
        .iter()
        .map(|offer| offer.format)
        .collect();
    offered.sort_unstable();
    assert_eq!(offered, [ARGB, XRGB], "linear ARGB8888 and XRGB8888");
    assert!(
        copy.dmabuf_offers()
            .iter()
            .all(|offer| (offer.size.w, offer.size.h) == (640, 480)),
        "at the output's mode size"
    );
    assert!(!copy.offers().is_empty(), "shared memory stays on offer");
}

/// Without a dma-buf global a `zwlr_screencopy` frame offers shared memory alone.
#[test]
fn a_wlr_frame_without_a_dma_buf_global_announces_none() {
    let mut fixture = Fixture::start(small()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut copy = ScreenCopy::start(fixture.client(client).globals()).expect("the manager");
    settle_copy(&mut fixture, &mut copy);
    copy.capture("HEADLESS-1", CopyArea::Output, CursorMode::Hidden)
        .expect("a frame");
    settle_copy(&mut fixture, &mut copy);
    assert!(copy.dmabuf_offers().is_empty());
    assert!(!copy.offers().is_empty());
}
