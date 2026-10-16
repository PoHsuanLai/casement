//! Where every layer surface on an output goes, computed once (wlr-layer-shell, in our words).
//!
//! The renderer, the hit test and the configure sizes all read [`Arranged`], so what is drawn,
//! what receives the pointer and what a client is told never disagree.
//!
//! Surfaces that reserve a zone are arranged first, overlay level down to background and in
//! mapping order (`LayerKey`) within a level; each is arranged inside the area the earlier ones
//! left, and then takes its strip from that area. A surface that asks to avoid zones (`0`), or
//! whose positive zone its anchoring does not make meaningful, is arranged inside what is left
//! at the end; one that ignores zones (`-1`) inside the whole output.

use super::{Anchor, Anchors, ExclusiveZone, LayerOwner, LayerSpec, Margins};
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use crate::ids::{LayerKey, OutputKey};
use std::cmp::Reverse;
use std::collections::BTreeMap;

/// Every layer surface of one output, each with the room it is arranged in, and what is left
/// for windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arranged {
    slots: BTreeMap<LayerKey, Slot>,
    usable: LogicalRect,
}

/// One surface's anchors, margins and room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slot {
    anchor: Anchors,
    margin: Margins,
    room: LogicalRect,
    /// The size the surface is configured to (0 on an axis it chooses).
    size: LogicalSize,
    /// Where it was put by hand, if it was.
    pin: Option<LogicalPoint>,
}

impl Arranged {
    /// What the exclusive zones leave for windows (the whole output when there is none).
    pub fn usable(&self) -> LogicalRect {
        self.usable
    }

    /// The room `layer` is arranged in: its surface is sized from it and placed inside it.
    pub fn room(&self, layer: &LayerKey) -> Option<LogicalRect> {
        self.slots.get(layer).map(|slot| slot.room)
    }

    /// The size `layer` is configured to: what it asked for on an axis, else all the room less
    /// the margins when it is held to both ends of that axis, else 0 (the surface chooses).
    pub fn configure_size(&self, layer: &LayerKey) -> Option<LogicalSize> {
        self.slots.get(layer).map(|slot| slot.size)
    }

    /// Where `layer` sits (global logical pixels) when its surface has `committed` size. The
    /// configured size decides where it sits, not the buffer (a smaller buffer sits at the
    /// corner of the box it was given, wlr-layer-shell); the committed size stands in on an
    /// axis the surface chooses.
    pub fn place(&self, layer: &LayerKey, committed: LogicalSize) -> Option<LogicalRect> {
        let slot = self.slots.get(layer)?;
        let own = |configured: u32, committed: u32| {
            if configured == 0 {
                committed
            } else {
                configured
            }
        };
        let size = LogicalSize {
            w: own(slot.size.w, committed.w),
            h: own(slot.size.h, committed.h),
        };
        let placed = place_in(slot.anchor, slot.margin, slot.room, size);
        Some(match slot.pin {
            Some(origin) => LogicalRect::new(origin, placed.size),
            None => placed,
        })
    }
}

impl super::LayerBook {
    /// The layers on `output`, whose whole rectangle is `full`, arranged.
    pub fn arrange(&self, output: &OutputKey, full: LogicalRect) -> Arranged {
        let mut here: Vec<(&LayerKey, &LayerSpec)> = self
            .iter()
            .filter(|(_, spec)| spec.output == *output)
            .collect();
        here.sort_by_key(|(key, spec)| (Reverse(spec.level), **key));
        let (reserving, others): (Vec<_>, Vec<_>) = here
            .into_iter()
            .partition(|(_, spec)| reservation(spec).is_some());
        let (usable, mut slots) = reserving.into_iter().fold(
            (full, BTreeMap::new()),
            |(usable, mut slots), (key, spec)| {
                slots.insert(*key, slot(spec, usable, self.pin(key)));
                let left = reservation(spec)
                    .map(|taken| allowed(spec.owner, taken, usable, full))
                    .map_or(usable, |taken| shrink(usable, taken));
                (left, slots)
            },
        );
        for (key, spec) in others {
            let room = match spec.exclusive {
                ExclusiveZone::Ignore => full,
                ExclusiveZone::Respect | ExclusiveZone::Reserve(_) => usable,
            };
            slots.insert(*key, slot(spec, room, self.pin(key)));
        }
        Arranged { slots, usable }
    }
}

fn slot(spec: &LayerSpec, room: LogicalRect, pin: Option<LogicalPoint>) -> Slot {
    Slot {
        pin,
        anchor: spec.anchor,
        margin: spec.margin,
        room,
        size: LogicalSize {
            w: configured(
                spec.size.w,
                (spec.anchor.left, spec.anchor.right),
                room.size.w,
                (spec.margin.left, spec.margin.right),
            ),
            h: configured(
                spec.size.h,
                (spec.anchor.top, spec.anchor.bottom),
                room.size.h,
                (spec.margin.top, spec.margin.bottom),
            ),
        },
    }
}

/// The length a surface is configured to along one axis: what it asked for when that is not 0,
/// else all the room less the margins when it is held to both ends, else 0.
fn configured(asked: u32, ends: (Anchor, Anchor), room: u32, margins: (i32, i32)) -> u32 {
    match (asked, ends) {
        (0, (Anchor::Held, Anchor::Held)) => {
            let taken = i64::from(margins.0) + i64::from(margins.1);
            u32::try_from((i64::from(room) - taken).max(0)).unwrap_or(0)
        }
        (asked, _) => asked,
    }
}

/// One reservation: this edge loses this many logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Reservation {
    edge: Edge,
    depth: u32,
}

/// A side of an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// What `spec` takes from which edge, if anything: its zone plus its margin on the one edge
/// its anchoring names (that edge held with the opposite edge free and the two perpendicular
/// edges both held or both free; any other anchoring, two adjacent edges, opposite edges, all
/// four, none, is ambiguous and reserves nothing, as does a zone that is not
/// [`ExclusiveZone::Reserve`]).
fn reservation(spec: &LayerSpec) -> Option<Reservation> {
    let ExclusiveZone::Reserve(zone) = spec.exclusive else {
        return None;
    };
    let edge = exclusive_edge(&spec.anchor)?;
    let margin = match edge {
        Edge::Top => spec.margin.top,
        Edge::Bottom => spec.margin.bottom,
        Edge::Left => spec.margin.left,
        Edge::Right => spec.margin.right,
    };
    let depth = u32::try_from(i64::from(zone) + i64::from(margin).max(0)).unwrap_or(u32::MAX);
    (depth > 0).then_some(Reservation { edge, depth })
}

/// The share of an output's width or height one client surface may reserve beside its edge, as
/// a divisor (a quarter): a bar or a dock never needs more, and a surface asking for the whole
/// axis would leave windows no room (CASC-10).
const CLIENT_ZONE_DIVISOR: u32 = 4;

/// The share of an axis, as a divisor (a half), that client zones leave to windows however many
/// of them there are: four client surfaces on the four edges cannot shrink the area to nothing.
const CLIENT_KEEPS_DIVISOR: u32 = 2;

/// `taken` as `owner` may have it. The shell's surfaces reserve what they ask for. Another
/// client's reserves at most a quarter of the output along the axis, and nothing that would
/// leave windows less than half of it (`usable` is what the earlier zones left, `full` the
/// whole output).
fn allowed(
    owner: LayerOwner,
    taken: Reservation,
    usable: LogicalRect,
    full: LogicalRect,
) -> Reservation {
    match owner {
        LayerOwner::Shell => taken,
        LayerOwner::Client => {
            let (whole, left) = match taken.edge {
                Edge::Top | Edge::Bottom => (full.size.h, usable.size.h),
                Edge::Left | Edge::Right => (full.size.w, usable.size.w),
            };
            let depth = taken
                .depth
                .min(whole / CLIENT_ZONE_DIVISOR)
                .min(left.saturating_sub(whole / CLIENT_KEEPS_DIVISOR));
            Reservation { depth, ..taken }
        }
    }
}

/// The one edge an anchoring reserves beside, if it names one.
fn exclusive_edge(anchor: &Anchors) -> Option<Edge> {
    let held = |a: Anchor| a == Anchor::Held;
    let (top, bottom, left, right) = (
        held(anchor.top),
        held(anchor.bottom),
        held(anchor.left),
        held(anchor.right),
    );
    let vertical_sides_agree = left == right;
    let horizontal_sides_agree = top == bottom;
    match (top, bottom, left, right) {
        (true, false, _, _) if vertical_sides_agree => Some(Edge::Top),
        (false, true, _, _) if vertical_sides_agree => Some(Edge::Bottom),
        (_, _, true, false) if horizontal_sides_agree => Some(Edge::Left),
        (_, _, false, true) if horizontal_sides_agree => Some(Edge::Right),
        _ => None,
    }
}

/// `area` without the strip `reservation` takes beside its edge (never more than is there).
fn shrink(area: LogicalRect, reservation: Reservation) -> LogicalRect {
    let LogicalRect { origin, size } = area;
    let (along_h, along_w) = (reservation.depth.min(size.h), reservation.depth.min(size.w));
    match reservation.edge {
        Edge::Top => LogicalRect::new(
            LogicalPoint {
                x: origin.x,
                y: origin.y.saturating_add_unsigned(along_h),
            },
            LogicalSize {
                w: size.w,
                h: size.h - along_h,
            },
        ),
        Edge::Bottom => LogicalRect::new(
            origin,
            LogicalSize {
                w: size.w,
                h: size.h - along_h,
            },
        ),
        Edge::Left => LogicalRect::new(
            LogicalPoint {
                x: origin.x.saturating_add_unsigned(along_w),
                y: origin.y,
            },
            LogicalSize {
                w: size.w - along_w,
                h: size.h,
            },
        ),
        Edge::Right => LogicalRect::new(
            origin,
            LogicalSize {
                w: size.w - along_w,
                h: size.h,
            },
        ),
    }
}

/// Where one axis of a surface sits: its start and length inside `[origin, origin + room)`.
/// Held at both ends it fills the room less the margins (a zero committed length means "as
/// much as there is"); held at one end it sits against it; otherwise it is centred.
fn along(
    room: (i32, u32),
    ends: (Anchor, Anchor),
    margins: (i32, i32),
    committed: u32,
) -> (i32, u32) {
    let (origin, space) = room;
    let (near, far) = margins;
    let end = |value: i64| i32::try_from(value).unwrap_or(i32::MAX);
    let length = |value: i64| u32::try_from(value.max(0)).unwrap_or(u32::MAX);
    match ends {
        (Anchor::Held, Anchor::Held) => {
            let span = i64::from(space) - i64::from(near) - i64::from(far);
            let own = if committed == 0 {
                length(span)
            } else {
                committed
            };
            let slack = (span - i64::from(own)).max(0);
            (end(i64::from(origin) + i64::from(near) + slack / 2), own)
        }
        (Anchor::Held, Anchor::Free) => (end(i64::from(origin) + i64::from(near)), committed),
        (Anchor::Free, Anchor::Held) => (
            end(i64::from(origin) + i64::from(space) - i64::from(far) - i64::from(committed)),
            committed,
        ),
        (Anchor::Free, Anchor::Free) => (
            end(i64::from(origin) + (i64::from(space) - i64::from(committed)) / 2),
            committed,
        ),
    }
}

/// The rectangle (global logical) a surface of `committed` size occupies in `room`, from its
/// anchors and margins.
pub fn place_in(
    anchor: Anchors,
    margin: Margins,
    room: LogicalRect,
    committed: LogicalSize,
) -> LogicalRect {
    let (x, w) = along(
        (room.origin.x, room.size.w),
        (anchor.left, anchor.right),
        (margin.left, margin.right),
        committed.w,
    );
    let (y, h) = along(
        (room.origin.y, room.size.h),
        (anchor.top, anchor.bottom),
        (margin.top, margin.bottom),
        committed.h,
    );
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}
