//! A tiled window's target: the half or quarter of the usable area its zone names, inside the
//! tile margin, with the window's frame (the titlebar above its content) inside the zone too.
//! The same rectangle is the preview a drag to an edge shows (`zone_rect`).

use super::desktop::below_chrome;
use super::floating::{fit_size, placed};
use crate::{LayoutReport, WindowTarget};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::window::SizeLimits;
use casement_core::{OutputKey, WindowId};
use casement_protocols::vocab::TileZone;

/// Which part of one axis a zone takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    /// All of it.
    Whole,
    /// The first half (left, top).
    First,
    /// The second half (right, bottom).
    Second,
}

/// The parts of the horizontal and the vertical axis `zone` takes.
fn parts(zone: TileZone) -> (Part, Part) {
    match zone {
        TileZone::Left => (Part::First, Part::Whole),
        TileZone::Right => (Part::Second, Part::Whole),
        TileZone::Top => (Part::Whole, Part::First),
        TileZone::Bottom => (Part::Whole, Part::Second),
        TileZone::TopLeft => (Part::First, Part::First),
        TileZone::TopRight => (Part::Second, Part::First),
        TileZone::BottomLeft => (Part::First, Part::Second),
        TileZone::BottomRight => (Part::Second, Part::Second),
    }
}

/// `part` of the span from `start` over `len`, with `margin` round it and between the halves.
/// An odd length gives the extra pixel to the second half. A margin larger than a quarter of the
/// span is held to it, so a tiny area is never turned inside out.
fn span(start: i32, len: u32, part: Part, margin: u32) -> (i32, u32) {
    let margin = margin.min(len / 4);
    let inner = len - 2 * margin;
    let first = (inner - margin) / 2;
    let (from, take) = match part {
        Part::Whole => (margin, inner),
        Part::First => (margin, first),
        Part::Second => (margin + first + margin, inner - margin - first),
    };
    let at = i64::from(start) + i64::from(from);
    (i32::try_from(at).unwrap_or(i32::MAX), take)
}

/// The rectangle `zone` takes of `area` (the usable area), `margin` logical pixels from the
/// area's edges and from the other half. The frame of a tiled window is inside it.
pub fn zone_rect(zone: TileZone, area: LogicalRect, margin: u32) -> LogicalRect {
    let (across, down) = parts(zone);
    let (x, w) = span(area.origin.x, area.size.w, across, margin);
    let (y, h) = span(area.origin.y, area.size.h, down, margin);
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

/// The target of a window tiled to `zone` of `area`: its content below its frame's `reserve`
/// rows, its size held to its limits (an unmet minimum is reported). Like a fill it asks nothing
/// of its own, so the size always goes out as a configure.
pub(super) fn tiled_target(
    window: WindowId,
    output: &OutputKey,
    zone: TileZone,
    limits: SizeLimits,
    reserve: u32,
    area: LogicalRect,
    margin: u32,
) -> (WindowTarget, Option<LayoutReport>) {
    let room = below_chrome(zone_rect(zone, area, margin), reserve);
    let size = fit_size(room.size, limits, room);
    placed(
        window,
        output,
        LogicalSize::ZERO,
        LogicalRect::new(room.origin, size),
        limits,
    )
}
