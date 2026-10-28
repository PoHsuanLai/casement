//! Where layers are arranged: the one placement the renderer, the hit test and the configure
//! sizes all read.

use super::{
    Anchors, ExclusiveZone, KeyboardInteractivity, LayerBook, LayerLevel, LayerOwner, LayerSpec,
    Margins, ShellRoleTag,
};
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use crate::ids::{LayerKey, OutputKey, OutputName};
use crate::output::{EdidFacts, EdidSerial};

pub(super) fn output(name: &str) -> OutputKey {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    OutputKey::of(&edid, &OutputName::parse(name).expect("name"))
}

pub(super) fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

const NONE: Margins = Margins {
    top: 0,
    right: 0,
    bottom: 0,
    left: 0,
};

/// One layer in a case: its level, edges ("t", "b", "l", "r"), zone, margins and the size its
/// surface committed.
pub(super) struct Ask {
    level: LayerLevel,
    edges: &'static str,
    zone: ExclusiveZone,
    margin: Margins,
    committed: (u32, u32),
    owner: LayerOwner,
}

pub(super) fn ask(edges: &'static str, zone: ExclusiveZone, committed: (u32, u32)) -> Ask {
    Ask {
        level: LayerLevel::Top,
        edges,
        zone,
        margin: NONE,
        committed,
        owner: LayerOwner::Shell,
    }
}

impl Ask {
    /// The same layer, from `owner`'s connection (the shell's by default).
    pub(super) fn owned_by(self, owner: LayerOwner) -> Ask {
        Ask { owner, ..self }
    }

    pub(super) fn spec(&self) -> LayerSpec {
        let held = |c: char| self.edges.contains(c);
        LayerSpec {
            output: output("HEADLESS-1"),
            level: self.level,
            anchor: Anchors {
                top: held('t'),
                bottom: held('b'),
                left: held('l'),
                right: held('r'),
            },
            exclusive: self.zone,
            margin: self.margin,
            keyboard: KeyboardInteractivity::None,
            namespace: "test".to_owned(),
            size: crate::geometry::LogicalSize::ZERO,
            role: ShellRoleTag::Unassigned,
            owner: self.owner,
        }
    }
}

/// The 1920x1080 output at (100, 50), as a case's layers (keys 1, 2, ...) place on it.
fn placed(asks: &[Ask]) -> Vec<LogicalRect> {
    let full = rect(100, 50, 1920, 1080);
    let book = asks
        .iter()
        .enumerate()
        .fold(LayerBook::empty(), |book, (n, ask)| {
            book.with_layer(LayerKey(n as u64 + 1), ask.spec())
        });
    let arranged = book.arrange(&output("HEADLESS-1"), full);
    asks.iter()
        .enumerate()
        .map(|(n, ask)| {
            let (w, h) = ask.committed;
            arranged
                .place(&LayerKey(n as u64 + 1), LogicalSize { w, h })
                .expect("arranged")
        })
        .collect()
}

#[test]
fn layers_are_placed_in_what_the_earlier_zones_left() {
    let reserve = ExclusiveZone::Reserve;
    let margin_top = Margins { top: 6, ..NONE };
    let cases: Vec<(&str, Vec<Ask>, Vec<LogicalRect>)> = vec![
        (
            "one bar sits at the top of the output",
            vec![ask("tlr", reserve(24), (1920, 24))],
            vec![rect(100, 50, 1920, 24)],
        ),
        (
            "two bars on one edge stack, the second below the first",
            vec![
                ask("tlr", reserve(24), (1920, 24)),
                ask("tlr", reserve(30), (1920, 30)),
            ],
            vec![rect(100, 50, 1920, 24), rect(100, 74, 1920, 30)],
        ),
        (
            "two bars on one edge, mapped in the other order, still stack by key",
            vec![
                ask("tlr", reserve(30), (1920, 30)),
                ask("tlr", reserve(24), (1920, 24)),
            ],
            vec![rect(100, 50, 1920, 30), rect(100, 80, 1920, 24)],
        ),
        (
            "a bar then a left dock: the dock starts below the bar",
            vec![
                ask("tlr", reserve(24), (1920, 24)),
                ask("ltb", reserve(64), (64, 1056)),
            ],
            vec![rect(100, 50, 1920, 24), rect(100, 74, 64, 1056)],
        ),
        (
            "a left dock then a bar: the bar starts right of the dock",
            vec![
                ask("ltb", reserve(64), (64, 1080)),
                ask("tlr", reserve(24), (1856, 24)),
            ],
            vec![rect(100, 50, 64, 1080), rect(164, 50, 1856, 24)],
        ),
        (
            "a bottom dock then a top bar: the bar sits in what the dock left",
            vec![
                ask("blr", reserve(80), (1920, 80)),
                ask("tlr", reserve(24), (1920, 24)),
            ],
            vec![rect(100, 1050, 1920, 80), rect(100, 50, 1920, 24)],
        ),
        (
            "a zone-0 layer is placed in what every zone left",
            vec![
                ask("tlr", reserve(24), (1920, 24)),
                ask("blr", reserve(80), (1920, 80)),
                ask("tr", ExclusiveZone::Respect, (300, 100)),
            ],
            vec![
                rect(100, 50, 1920, 24),
                rect(100, 1050, 1920, 80),
                rect(1720, 74, 300, 100),
            ],
        ),
        (
            "a zone-0 layer mapped first is still placed after the zones",
            vec![
                ask("tr", ExclusiveZone::Respect, (300, 100)),
                ask("tlr", reserve(24), (1920, 24)),
            ],
            vec![rect(1720, 74, 300, 100), rect(100, 50, 1920, 24)],
        ),
        (
            "a layer that ignores zones is placed in the whole output",
            vec![
                ask("tlr", reserve(24), (1920, 24)),
                ask("tr", ExclusiveZone::Ignore, (300, 100)),
            ],
            vec![rect(100, 50, 1920, 24), rect(1720, 50, 300, 100)],
        ),
        (
            "a margin on the reserved edge is part of what the bar takes",
            vec![
                Ask {
                    margin: margin_top,
                    ..ask("tlr", reserve(24), (1920, 24))
                },
                ask("tlr", reserve(30), (1920, 30)),
            ],
            vec![rect(100, 56, 1920, 24), rect(100, 80, 1920, 30)],
        ),
        (
            "an overlay bar is arranged before a top bar",
            vec![
                ask("tlr", reserve(24), (1920, 24)),
                Ask {
                    level: LayerLevel::Overlay,
                    ..ask("tlr", reserve(30), (1920, 30))
                },
            ],
            vec![rect(100, 80, 1920, 24), rect(100, 50, 1920, 30)],
        ),
        (
            "an ambiguous anchoring reserves nothing (the bar is not pushed) and is placed in what is left",
            vec![
                ask("tl", reserve(50), (50, 50)),
                ask("tlr", reserve(24), (1920, 24)),
            ],
            vec![rect(100, 74, 50, 50), rect(100, 50, 1920, 24)],
        ),
    ];
    for (name, asks, want) in cases {
        assert_eq!(placed(&asks), want, "{name}");
    }
}

#[test]
fn what_is_left_for_windows_is_what_the_layers_were_arranged_in() {
    let full = rect(100, 50, 1920, 1080);
    let book = [
        ask("tlr", ExclusiveZone::Reserve(24), (0, 24)),
        ask("tlr", ExclusiveZone::Reserve(30), (0, 30)),
        ask("ltb", ExclusiveZone::Reserve(64), (64, 0)),
    ]
    .iter()
    .enumerate()
    .fold(LayerBook::empty(), |book, (n, ask)| {
        book.with_layer(LayerKey(n as u64 + 1), ask.spec())
    });
    let arranged = book.arrange(&output("HEADLESS-1"), full);
    assert_eq!(arranged.usable(), rect(164, 104, 1856, 1026));
    assert_eq!(arranged.room(&LayerKey(2)), Some(rect(100, 74, 1920, 1056)));
    assert_eq!(
        arranged.room(&LayerKey(3)),
        Some(rect(100, 104, 1920, 1026))
    );
    assert_eq!(arranged.room(&LayerKey(9)), None);
    assert_eq!(
        book.usable_area(&output("HEADLESS-1"), full),
        arranged.usable()
    );
}

#[test]
fn anchors_and_margins_place_a_surface() {
    let screen = rect(100, 50, 1920, 1080);
    let bar = Anchors {
        top: true,
        bottom: false,
        left: true,
        right: true,
    };
    let corner = Anchors {
        top: false,
        bottom: true,
        left: false,
        right: true,
    };
    let none = Anchors {
        top: false,
        bottom: false,
        left: false,
        right: false,
    };
    let all = Anchors {
        top: true,
        bottom: true,
        left: true,
        right: true,
    };
    let margin = Margins {
        top: 8,
        right: 10,
        bottom: 12,
        left: 6,
    };
    let size = |w, h| LogicalSize { w, h };
    let cases = [
        (
            "a bar spans the width at the top",
            bar,
            NONE,
            size(0, 30),
            rect(100, 50, 1920, 30),
        ),
        (
            "margins shrink a spanning bar and push it off the edge",
            bar,
            margin,
            size(0, 30),
            rect(106, 58, 1904, 30),
        ),
        (
            "a corner layer sits against both its edges",
            corner,
            margin,
            size(200, 100),
            rect(100 + 1920 - 10 - 200, 50 + 1080 - 12 - 100, 200, 100),
        ),
        (
            "no anchor centres",
            none,
            NONE,
            size(400, 200),
            rect(100 + 760, 50 + 440, 400, 200),
        ),
        (
            "a full anchor with a committed size centres inside the margins",
            all,
            NONE,
            size(320, 240),
            rect(100 + 800, 50 + 420, 320, 240),
        ),
        (
            "a full anchor with no size fills the output",
            all,
            NONE,
            size(0, 0),
            screen,
        ),
        (
            "an oversized surface held at both ends starts at its margin",
            all,
            Margins { top: 6, ..NONE },
            size(40, 2000),
            rect(100 + 940, 56, 40, 2000),
        ),
    ];
    for (name, anchor, margin, committed, want) in cases {
        assert_eq!(
            super::place_in(anchor, margin, screen, committed),
            want,
            "{name}"
        );
    }
}

/// Each bar's hit box is its own placed box, not the output: a point in one bar is in no other.
#[test]
fn bars_stacked_on_one_edge_each_own_their_box() {
    let reserve = ExclusiveZone::Reserve;
    let boxes = placed(&[
        ask("tlr", reserve(24), (1920, 24)),
        ask("tlr", reserve(30), (1920, 30)),
        ask("blr", reserve(80), (1920, 80)),
    ]);
    let full = rect(100, 50, 1920, 1080);
    assert!(boxes.iter().all(|placed| placed.size.h < full.size.h));
    // (x, y, the one box that owns the point)
    let probes = [
        (500, 60, Some(0)),
        (500, 90, Some(1)),
        (500, 600, None),
        (500, 1100, Some(2)),
    ];
    for (x, y, owner) in probes {
        let at = LogicalPoint { x, y };
        let owners: Vec<usize> = boxes
            .iter()
            .enumerate()
            .filter(|(_, placed)| placed.contains(at))
            .map(|(n, _)| n)
            .collect();
        assert_eq!(owners, owner.into_iter().collect::<Vec<_>>(), "({x}, {y})");
    }
}
