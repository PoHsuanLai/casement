//! What a layer surface is configured to and where it sits when it asked for a size, and a
//! surface put by hand.

use super::arrange_tests::{ask, output, rect};
use super::{ExclusiveZone, LayerBook, LayerSpec};
use crate::geometry::{LogicalPoint, LogicalSize};
use crate::ids::LayerKey;

/// A top-layer spec with `edges` held and `size` asked for.
fn sized(edges: &'static str, size: (u32, u32)) -> LayerSpec {
    LayerSpec {
        size: LogicalSize {
            w: size.0,
            h: size.1,
        },
        ..ask(edges, ExclusiveZone::Respect, (0, 0)).spec()
    }
}

#[test]
fn a_surface_is_configured_to_what_it_asked_for_else_to_the_room() {
    let full = rect(100, 50, 1920, 1080);
    // name, edges, asked, configured
    type Case = (&'static str, &'static str, (u32, u32), (u32, u32));
    let cases: [Case; 5] = [
        (
            "an unanchored surface keeps its size",
            "",
            (123, 546),
            (123, 546),
        ),
        (
            "an explicit size wins over being held to every edge",
            "tblr",
            (321, 218),
            (321, 218),
        ),
        (
            "held to every edge it fills the room",
            "tblr",
            (0, 0),
            (1920, 1080),
        ),
        (
            "held to both sides it fills the width only",
            "lr",
            (0, 40),
            (1920, 40),
        ),
        (
            "no ask and no edge pair leaves the choice to it",
            "l",
            (0, 0),
            (0, 0),
        ),
    ];
    for (case, edges, asked, want) in cases {
        let book = LayerBook::empty().with_layer(LayerKey(1), sized(edges, asked));
        let arranged = book.arrange(&output("HEADLESS-1"), full);
        let got = arranged.configure_size(&LayerKey(1)).expect("arranged");
        assert_eq!((got.w, got.h), want, "{case}");
    }
}

#[test]
fn a_surface_sits_where_its_configured_box_is_not_where_its_buffer_would_centre() {
    let full = rect(0, 0, 1920, 1080);
    let book = LayerBook::empty().with_layer(LayerKey(1), sized("br", (200, 300)));
    let arranged = book.arrange(&output("HEADLESS-1"), full);
    // A 52x74 buffer in a 200x300 box: the box is against the corner, the buffer at its top-left.
    let at = arranged
        .place(&LayerKey(1), LogicalSize { w: 52, h: 74 })
        .expect("arranged");
    assert_eq!(at, rect(1720, 780, 200, 300));
}

#[test]
fn a_pinned_surface_keeps_its_size_and_sits_at_the_pin() {
    let full = rect(0, 0, 1920, 1080);
    let book = LayerBook::empty()
        .with_layer(LayerKey(1), sized("", (200, 100)))
        .with_pin(LayerKey(1), LogicalPoint { x: 7, y: 9 });
    let arranged = book.arrange(&output("HEADLESS-1"), full);
    let size = LogicalSize { w: 0, h: 0 };
    assert_eq!(
        arranged.place(&LayerKey(1), size),
        Some(rect(7, 9, 200, 100))
    );
    // Unmapped, the pin goes with it.
    let book = book.without_layer(&LayerKey(1));
    assert_eq!(book.pin(&LayerKey(1)), None);
}
