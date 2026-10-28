//! `pass::capture_output` on pixman: the capture is the frame the render drew, byte for byte.
#![cfg(feature = "testing")]

use crate::support;

use casement_core::geometry::PhysicalRect;
use casement_render::pass::{CaptureFormat, CaptureRequest, CapturedFrame};
use casement_render::{CasementElement, Rgba8, solid_rect};
use smithay::backend::renderer::Bind;
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::backend::renderer::utils::CommitCounter;
use support::{Offscreen1, pixman, tracker};

const SIZE: (i32, i32) = (64, 48);

fn rect(x: i32, y: i32, w: u32, h: u32) -> PhysicalRect {
    PhysicalRect { x, y, w, h }
}

/// A drawn target: its read-back and a capture request's answer for `area`.
fn captured(area: PhysicalRect) -> (Vec<u8>, CapturedFrame) {
    let mut renderer = pixman();
    let mut target = renderer.target(SIZE);
    let mut tracker = tracker(SIZE, 1.0);
    let elements: Vec<CasementElement<PixmanRenderer>> = vec![
        CasementElement::Solid(solid_rect(
            Id::new(),
            rect(8, 6, 20, 10),
            Rgba8 {
                r: 200,
                g: 40,
                b: 90,
                a: 255,
            },
            CommitCounter::default(),
        )),
        CasementElement::Solid(solid_rect(
            Id::new(),
            rect(0, 0, 64, 48),
            Rgba8 {
                r: 10,
                g: 20,
                b: 30,
                a: 255,
            },
            CommitCounter::default(),
        )),
    ];
    let (_, scanout) = renderer.draw(&mut target, &mut tracker, SIZE, &elements, 0);
    let framebuffer = renderer.bind(&mut target).expect("bind");
    let request = CaptureRequest::new(area, false);
    let frame = casement_render::pass::capture_output(&mut renderer, &framebuffer, &request)
        .expect("capture");
    (scanout, frame)
}

#[test]
fn a_whole_output_capture_is_the_scanout_frame() {
    let whole = rect(0, 0, 64, 48);
    let (scanout, frame) = captured(whole);
    assert_eq!(frame, CapturedFrame::from_rgba(whole, &scanout));
    assert_eq!(frame.format, CaptureFormat::Argb8888);
    assert_eq!(frame.stride, 64 * 4);
}

#[test]
fn a_region_capture_is_the_same_rows_cut_out() {
    let (scanout, frame) = captured(rect(4, 2, 10, 5));
    let expected: Vec<u8> = (2..7_usize)
        .flat_map(|row| {
            let from = (row * 64 + 4) * 4;
            scanout[from..from + 40].to_vec()
        })
        .collect();
    assert_eq!(
        frame.bytes,
        CapturedFrame::from_rgba(frame.area, &expected).bytes
    );
}
