//! Where each layer-shell surface sits on its output: anchored edges, margins and exclusive
//! zones as the client committed them (wlr-layer-shell, in our words). Exclusive surfaces are
//! placed first, overlay level down to background, each taking its zone from the usable area;
//! surfaces that respect zones are then placed inside what is left, and those that ignore them
//! inside the whole output. A surface is drawn at its committed size.

use crate::scene::LayerDraw;
use crate::texture_book::TextureBook;
use casement_core::LayerKey;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::layer::{Anchor, Anchors, ExclusiveZone, Margins};
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::wlr_layer::{self, LayerSurfaceCachedState};
use std::collections::BTreeMap;

/// What one layer surface committed, converted at the edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LayerAsk {
    /// The edges it is held to.
    pub(crate) anchors: Anchors,
    /// Its distance from each edge.
    pub(crate) margin: Margins,
    /// The space it keeps for itself.
    pub(crate) zone: ExclusiveZone,
    /// Its committed size.
    pub(crate) size: LogicalSize,
}

/// Every layer surface in `layers` that `book` can draw, placed on the output `full`.
pub(crate) fn place_layers(
    layers: &LayerDraw,
    book: &TextureBook,
    full: LogicalRect,
) -> BTreeMap<LayerKey, LogicalRect> {
    let asks: Vec<(LayerKey, LayerAsk)> = [
        &layers.overlay,
        &layers.top,
        &layers.bottom,
        &layers.background,
    ]
    .into_iter()
    .flatten()
    .filter_map(|key| {
        let entry = book.layer(key)?;
        let state = with_states(&entry.root, |states| {
            *states
                .cached_state
                .get::<LayerSurfaceCachedState>()
                .current()
        });
        Some((*key, ask_of(&state, entry.committed)))
    })
    .collect();
    place_asks(&asks, full)
}

/// `asks` placed on `full` (the pure part of [`place_layers`]).
pub(crate) fn place_asks(
    asks: &[(LayerKey, LayerAsk)],
    full: LogicalRect,
) -> BTreeMap<LayerKey, LogicalRect> {
    let (exclusive, others): (Vec<_>, Vec<_>) =
        asks.iter().partition(|(_, ask)| reservation(ask).is_some());
    let (usable, mut placed) = exclusive.into_iter().fold(
        (full, BTreeMap::new()),
        |(usable, mut placed), (key, ask)| {
            placed.insert(key, place(&ask, usable));
            let shrunk =
                reservation(&ask).map_or(usable, |(edge, depth)| shrink(usable, edge, depth));
            (shrunk, placed)
        },
    );
    for (key, ask) in others {
        let bounds = match ask.zone {
            ExclusiveZone::Ignore => full,
            ExclusiveZone::Respect | ExclusiveZone::Reserve(_) => usable,
        };
        placed.insert(key, place(&ask, bounds));
    }
    placed
}

/// smithay's cached state as ours.
fn ask_of(state: &LayerSurfaceCachedState, size: LogicalSize) -> LayerAsk {
    let held = |edge: wlr_layer::Anchor| match state.anchor.contains(edge) {
        true => Anchor::Held,
        false => Anchor::Free,
    };
    LayerAsk {
        anchors: Anchors {
            top: held(wlr_layer::Anchor::TOP),
            bottom: held(wlr_layer::Anchor::BOTTOM),
            left: held(wlr_layer::Anchor::LEFT),
            right: held(wlr_layer::Anchor::RIGHT),
        },
        margin: Margins {
            top: state.margin.top,
            right: state.margin.right,
            bottom: state.margin.bottom,
            left: state.margin.left,
        },
        zone: match state.exclusive_zone {
            wlr_layer::ExclusiveZone::Exclusive(depth) => ExclusiveZone::Reserve(depth),
            wlr_layer::ExclusiveZone::Neutral => ExclusiveZone::Respect,
            wlr_layer::ExclusiveZone::DontCare => ExclusiveZone::Ignore,
        },
        size,
    }
}

/// A side of the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// The edge and depth an ask reserves: its one held edge (opposite free, the perpendicular
/// pair both held or both free) plus its margin there; none for any other anchoring.
fn reservation(ask: &LayerAsk) -> Option<(Edge, i64)> {
    let ExclusiveZone::Reserve(zone) = ask.zone else {
        return None;
    };
    let a = ask.anchors;
    let pair = |x: Anchor, y: Anchor| x == y;
    let edge = match (a.top, a.bottom, a.left, a.right) {
        (Anchor::Held, Anchor::Free, l, r) if pair(l, r) => Edge::Top,
        (Anchor::Free, Anchor::Held, l, r) if pair(l, r) => Edge::Bottom,
        (t, b, Anchor::Held, Anchor::Free) if pair(t, b) => Edge::Left,
        (t, b, Anchor::Free, Anchor::Held) if pair(t, b) => Edge::Right,
        _ => return None,
    };
    let margin = match edge {
        Edge::Top => ask.margin.top,
        Edge::Bottom => ask.margin.bottom,
        Edge::Left => ask.margin.left,
        Edge::Right => ask.margin.right,
    };
    Some((edge, i64::from(zone) + i64::from(margin)))
}

/// `area` without `depth` beside `edge` (never below nothing).
fn shrink(area: LogicalRect, edge: Edge, depth: i64) -> LogicalRect {
    let (x, y) = (i64::from(area.origin.x), i64::from(area.origin.y));
    let (w, h) = (i64::from(area.size.w), i64::from(area.size.h));
    let (x, y, w, h) = match edge {
        Edge::Top => (x, y + depth.clamp(0, h), w, h - depth.clamp(0, h)),
        Edge::Bottom => (x, y, w, h - depth.clamp(0, h)),
        Edge::Left => (x + depth.clamp(0, w), y, w - depth.clamp(0, w), h),
        Edge::Right => (x, y, w - depth.clamp(0, w), h),
    };
    rect(x, y, w, h)
}

/// `ask` placed inside `bounds`: held to its anchored edges by its margins, centred on an
/// axis held at both ends (an oversized surface starts at its near margin, as
/// `casement_wayland::layer_shell::layer_area` places it) or at neither.
fn place(ask: &LayerAsk, bounds: LogicalRect) -> LogicalRect {
    let size = (i64::from(ask.size.w), i64::from(ask.size.h));
    let x = axis(
        (i64::from(bounds.origin.x), i64::from(bounds.size.w)),
        (ask.anchors.left, ask.anchors.right),
        (i64::from(ask.margin.left), i64::from(ask.margin.right)),
        size.0,
    );
    let y = axis(
        (i64::from(bounds.origin.y), i64::from(bounds.size.h)),
        (ask.anchors.top, ask.anchors.bottom),
        (i64::from(ask.margin.top), i64::from(ask.margin.bottom)),
        size.1,
    );
    rect(x, y, size.0, size.1)
}

/// One axis of [`place`]: `(start, extent)` of the bounds, the two anchors and margins, the
/// surface's extent.
fn axis(bounds: (i64, i64), anchors: (Anchor, Anchor), margins: (i64, i64), extent: i64) -> i64 {
    let (start, room) = bounds;
    match anchors {
        (Anchor::Held, Anchor::Free) => start + margins.0,
        (Anchor::Free, Anchor::Held) => start + room - margins.1 - extent,
        (Anchor::Held, Anchor::Held) => {
            start + margins.0 + (room - margins.0 - margins.1 - extent).max(0) / 2
        }
        (Anchor::Free, Anchor::Free) => start + (room - extent) / 2,
    }
}

fn rect(x: i64, y: i64, w: i64, h: i64) -> LogicalRect {
    let i = |n: i64| i32::try_from(n).unwrap_or(if n < 0 { i32::MIN } else { i32::MAX });
    let u = |n: i64| u32::try_from(n.max(0)).unwrap_or(u32::MAX);
    LogicalRect::new(
        LogicalPoint { x: i(x), y: i(y) },
        LogicalSize { w: u(w), h: u(h) },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: Anchor = Anchor::Held;
    const F: Anchor = Anchor::Free;

    fn ask(
        anchors: (Anchor, Anchor, Anchor, Anchor),
        zone: ExclusiveZone,
        w: u32,
        h: u32,
    ) -> LayerAsk {
        LayerAsk {
            anchors: Anchors {
                top: anchors.0,
                bottom: anchors.1,
                left: anchors.2,
                right: anchors.3,
            },
            margin: Margins {
                top: 0,
                right: 0,
                bottom: 0,
                left: 0,
            },
            zone,
            size: LogicalSize { w, h },
        }
    }

    fn key(n: u64) -> LayerKey {
        LayerKey(n)
    }

    fn full() -> LogicalRect {
        rect(100, 0, 1280, 800)
    }

    #[test]
    fn anchors_and_margins_place_a_surface() {
        // name, anchors (top, bottom, left, right), margins (top, right, bottom, left), size, at
        type Case = (
            &'static str,
            (Anchor, Anchor, Anchor, Anchor),
            (i32, i32, i32, i32),
            (u32, u32),
            (i32, i32),
        );
        const CASES: &[Case] = &[
            (
                "bar on top",
                (H, F, H, H),
                (0, 0, 0, 0),
                (1280, 24),
                (100, 0),
            ),
            (
                "dock at the bottom, lifted",
                (F, H, H, H),
                (0, 0, 8, 0),
                (600, 64),
                (440, 728),
            ),
            (
                "top-left corner",
                (H, F, H, F),
                (5, 0, 0, 7),
                (50, 50),
                (107, 5),
            ),
            (
                "bottom-right corner",
                (F, H, F, H),
                (0, 9, 4, 0),
                (50, 50),
                (1321, 746),
            ),
            (
                "centred, no anchors",
                (F, F, F, F),
                (0, 0, 0, 0),
                (200, 100),
                (640, 350),
            ),
            (
                "left side, centred vertically",
                (H, H, H, F),
                (0, 0, 0, 0),
                (40, 400),
                (100, 200),
            ),
        ];
        for (name, anchors, (t, r, b, l), (w, h), at) in CASES {
            let mut asked = ask(*anchors, ExclusiveZone::Respect, *w, *h);
            asked.margin = Margins {
                top: *t,
                right: *r,
                bottom: *b,
                left: *l,
            };
            let placed = place_asks(&[(key(1), asked)], full());
            assert_eq!(
                placed[&key(1)].origin,
                LogicalPoint { x: at.0, y: at.1 },
                "{name}"
            );
        }
    }

    #[test]
    fn exclusive_zones_push_respecting_surfaces_and_not_ignoring_ones() {
        let asks = [
            (
                key(1),
                ask((H, F, H, H), ExclusiveZone::Reserve(24), 1280, 24),
            ),
            (
                key(2),
                ask((F, H, H, H), ExclusiveZone::Reserve(70), 600, 64),
            ),
            (key(3), ask((H, F, F, H), ExclusiveZone::Respect, 300, 80)),
            (key(4), ask((H, H, H, H), ExclusiveZone::Ignore, 1280, 800)),
            (
                key(5),
                ask((H, H, H, F), ExclusiveZone::Reserve(30), 30, 706),
            ),
        ];
        let placed = place_asks(&asks, full());
        assert_eq!(placed[&key(1)].origin, LogicalPoint { x: 100, y: 0 });
        // The side panel is placed after the bar took the top 24 and the dock the bottom 70.
        assert_eq!(placed[&key(5)].origin, LogicalPoint { x: 100, y: 24 });
        // The note respects all three zones: right edge at 1380, below the bar.
        assert_eq!(placed[&key(3)].origin, LogicalPoint { x: 1080, y: 24 });
        assert_eq!(placed[&key(4)].origin, LogicalPoint { x: 100, y: 0 });
    }

    #[test]
    fn ambiguous_anchoring_reserves_nothing() {
        let asks = [
            (
                key(6),
                ask((H, F, H, F), ExclusiveZone::Reserve(50), 50, 50),
            ),
            (key(3), ask((H, F, H, F), ExclusiveZone::Respect, 10, 10)),
        ];
        let placed = place_asks(&asks, full());
        assert_eq!(placed[&key(3)].origin, LogicalPoint { x: 100, y: 0 });
    }

    #[test]
    fn an_oversized_surface_held_at_both_ends_starts_at_its_margin() {
        let mut tall = ask((H, H, H, F), ExclusiveZone::Respect, 40, 900);
        tall.margin.top = 6;
        let placed = place_asks(&[(key(1), tall)], full());
        assert_eq!(placed[&key(1)].origin, LogicalPoint { x: 100, y: 6 });
    }
}
