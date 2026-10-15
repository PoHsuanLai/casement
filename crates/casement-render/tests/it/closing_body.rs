//! A closed window's last image on pixman: drawn where the closing content is, stretched to it,
//! at the window's opacity.
#![cfg(feature = "testing")]

use crate::support;

use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, PhysicalSize};
use casement_render::{CasementElement, ClosingBody, OutputView};
use ds_core::geometry::scale::Scale;
use ds_core::vocab::Fraction;
use smithay::backend::renderer::pixman::PixmanRenderer;
use support::{pixman, render_once};

const VIEW: OutputView = OutputView {
    area: LogicalRect {
        origin: LogicalPoint { x: 0, y: 0 },
        size: LogicalSize { w: 8, h: 6 },
    },
    scale: Scale(120),
    mode: PhysicalSize { w: 8, h: 6 },
};

/// A 2x2 body of one colour (R, G, B, A, premultiplied).
fn body(rgba: [u8; 4]) -> ClosingBody {
    let bytes: Vec<u8> = (0..4).flat_map(|_| rgba).collect();
    ClosingBody::from_rgba(1, PhysicalSize { w: 2, h: 2 }, &bytes, (0, Scale(120))).expect("a body")
}

fn pixel(bytes: &[u8], x: usize, y: usize) -> [u8; 4] {
    let at = (y * 8 + x) * 4;
    [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]
}

fn drawn(opacity: u16) -> Vec<u8> {
    let mut renderer: PixmanRenderer = pixman();
    let content = LogicalRect::new(LogicalPoint { x: 2, y: 1 }, LogicalSize { w: 4, h: 4 });
    let element: CasementElement<PixmanRenderer> = body([200, 40, 90, 255])
        .element(content, Fraction(opacity), &VIEW, &mut renderer)
        .expect("the renderer takes the body");
    render_once(&mut renderer, (8, 6), 1.0, &[element])
}

#[test]
fn the_body_is_stretched_over_the_closing_content() {
    let bytes = drawn(1_000);
    // The 2x2 image fills the 4x4 content at (2, 1); outside it the output is black.
    assert_eq!(
        pixel(&bytes, 2, 1),
        [200, 40, 90, 255],
        "top left of the body"
    );
    assert_eq!(
        pixel(&bytes, 5, 4),
        [200, 40, 90, 255],
        "bottom right of the body"
    );
    assert_eq!(pixel(&bytes, 1, 1), [0, 0, 0, 255], "left of it");
    assert_eq!(pixel(&bytes, 6, 4), [0, 0, 0, 255], "right of it");
}

#[test]
fn the_body_fades_with_the_window() {
    let bytes = drawn(500);
    let [r, g, b, a] = pixel(&bytes, 3, 2);
    let near = |got: u8, want: i16| (i16::from(got) - want).abs() <= 3;
    assert!(
        near(r, 100) && near(g, 20) && near(b, 45),
        "half over black: {r} {g} {b}"
    );
    assert_eq!(a, 255, "the output stays opaque");
}
