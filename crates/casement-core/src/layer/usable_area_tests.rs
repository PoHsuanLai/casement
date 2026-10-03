//! Properties of the usable area (the table is in `book.rs`).

use super::{
    Anchor, Anchors, ExclusiveZone, KeyboardInteractivity, LayerBook, LayerLevel, LayerSpec,
    Margins, ShellRoleTag,
};
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use crate::ids::{LayerKey, OutputKey, OutputName};
use crate::output::{EdidFacts, EdidSerial};
use proptest::prelude::*;

fn output(name: &str) -> OutputKey {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    OutputKey::of(&edid, &OutputName::parse(name).expect("name"))
}

fn anchor(held: bool) -> Anchor {
    if held { Anchor::Held } else { Anchor::Free }
}

fn spec(on: &str, edges: [bool; 4], zone: u32, margin: i32) -> LayerSpec {
    LayerSpec {
        output: output(on),
        level: LayerLevel::Top,
        anchor: Anchors {
            top: anchor(edges[0]),
            bottom: anchor(edges[1]),
            left: anchor(edges[2]),
            right: anchor(edges[3]),
        },
        exclusive: ExclusiveZone::Reserve(zone),
        margin: Margins {
            top: margin,
            right: margin,
            bottom: margin,
            left: margin,
        },
        keyboard: KeyboardInteractivity::None,
        namespace: "test".to_owned(),
        role: ShellRoleTag::Unassigned,
    }
}

fn inside(inner: LogicalRect, outer: LogicalRect) -> bool {
    inner.intersection(&outer) == Some(inner) || inner.size.w == 0 || inner.size.h == 0
}

proptest! {
    #[test]
    fn the_usable_area_stays_inside_the_output_and_never_grows(
        layers in prop::collection::vec(
            (prop::array::uniform4(any::<bool>()), 0u32..3000, -50i32..200, 1u8..3), 0..6),
        x in -2000i32..2000, y in -2000i32..2000, w in 0u32..4000, h in 0u32..3000,
    ) {
        let full = LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h });
        let book = layers.iter().enumerate().fold(LayerBook::empty(), |book, (n, (edges, zone, margin, on))| {
            book.with_layer(LayerKey(n as u64), spec(&format!("HEADLESS-{on}"), *edges, *zone, *margin))
        });
        let usable = book.usable_area(&output("HEADLESS-1"), full);
        prop_assert!(usable.size.w <= w && usable.size.h <= h);
        prop_assert!(inside(usable, full), "{usable:?} not in {full:?}");
    }

    #[test]
    fn reservations_on_one_edge_add_up_in_any_order(
        zones in prop::collection::vec(0u32..80, 1..5),
    ) {
        let full = LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 1920, h: 1080 });
        let book = |order: Vec<u32>| order.iter().enumerate().fold(LayerBook::empty(), |book, (n, zone)| {
            book.with_layer(LayerKey(n as u64), spec("HEADLESS-1", [true, false, true, true], *zone, 0))
        });
        let mut reversed = zones.clone();
        reversed.reverse();
        let forward = book(zones.clone()).usable_area(&output("HEADLESS-1"), full);
        prop_assert_eq!(forward, book(reversed).usable_area(&output("HEADLESS-1"), full));
        prop_assert_eq!(forward.size.h, 1080 - zones.iter().sum::<u32>());
    }
}
