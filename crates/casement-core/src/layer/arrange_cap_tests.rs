//! What a client's exclusive zone may take from windows (CASC-10): the shell's surfaces reserve
//! what they ask for; another client's at most a quarter of the axis, and all of them together
//! leave windows half of it.

use super::arrange_tests::{ask, output, rect};
use super::{ExclusiveZone, LayerBook, LayerOwner};
use crate::geometry::LogicalRect;
use crate::ids::LayerKey;

/// What is left for windows on a 1920x1080 output at (100, 50) after the asks, one per key.
fn usable(asks: &[(&'static str, u32, LayerOwner)]) -> LogicalRect {
    let book =
        asks.iter()
            .enumerate()
            .fold(LayerBook::empty(), |book, (n, (edges, zone, owner))| {
                let ask = ask(*edges, ExclusiveZone::Reserve(*zone), (0, 0)).owned_by(*owner);
                book.with_layer(LayerKey(n as u64 + 1), ask.spec())
            });
    book.usable_area(&output("HEADLESS-1"), rect(100, 50, 1920, 1080))
}

#[test]
fn a_client_zone_is_capped_and_the_shells_is_not() {
    // name, asks (edges, zone, whose), what is left for windows
    type Case = (
        &'static str,
        Vec<(&'static str, u32, LayerOwner)>,
        LogicalRect,
    );
    let cases: Vec<Case> = vec![
        (
            "a client bar of a sensible size gets it",
            vec![("tlr", 28, LayerOwner::Client)],
            rect(100, 78, 1920, 1052),
        ),
        (
            "a client asking for the whole axis gets a quarter",
            vec![("tlr", u32::MAX, LayerOwner::Client)],
            rect(100, 320, 1920, 810),
        ),
        (
            "the shell asking for the same gets what it asked for",
            vec![("tlr", 600, LayerOwner::Shell)],
            rect(100, 650, 1920, 480),
        ),
        (
            "a left client dock is held to a quarter of the width",
            vec![("ltb", 5000, LayerOwner::Client)],
            rect(580, 50, 1440, 1080),
        ),
        (
            "client bars stacked on one edge leave half the height",
            vec![
                ("tlr", 270, LayerOwner::Client),
                ("tlr", 270, LayerOwner::Client),
                ("tlr", 270, LayerOwner::Client),
            ],
            rect(100, 590, 1920, 540),
        ),
        (
            "four client surfaces on four edges leave half of each axis",
            vec![
                ("tlr", u32::MAX, LayerOwner::Client),
                ("blr", u32::MAX, LayerOwner::Client),
                ("ltb", u32::MAX, LayerOwner::Client),
                ("rtb", u32::MAX, LayerOwner::Client),
            ],
            rect(580, 320, 960, 540),
        ),
    ];
    for (name, asks, want) in cases {
        assert_eq!(usable(&asks), want, "{name}");
    }
}
