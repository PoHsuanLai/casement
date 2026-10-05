//! An interactive move or resize the person is doing: where the pointer was when it began, the
//! window's rectangle then, and what a pointer position makes of that rectangle. The seat
//! holds the one grab there can be; the core loop starts it (`Effect::StartGrab`), feeds it
//! the pointer and ends it at a button release.

use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::{GrabSpec, ResizeEdges, WindowId};

/// The smallest a resize makes a window.
const SMALLEST: u32 = 1;

/// A grab in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drag {
    /// What was asked.
    pub spec: GrabSpec,
    /// Where the pointer was when it began.
    pub start: LogicalPoint,
    /// The window's rectangle when the first pointer motion arrived (the layout has settled by
    /// then); none before.
    pub origin: Option<LogicalRect>,
}

impl Drag {
    /// A grab of `spec` that began with the pointer at `start`.
    pub fn begin(spec: GrabSpec, start: LogicalPoint) -> Drag {
        Drag {
            spec,
            start,
            origin: None,
        }
    }

    /// The window it moves.
    pub fn window(&self) -> WindowId {
        match self.spec {
            GrabSpec::Move { window, .. } | GrabSpec::Resize { window, .. } => window,
        }
    }

    /// The same grab, knowing the rectangle the window had.
    pub fn from_rect(self, origin: LogicalRect) -> Drag {
        Drag {
            origin: Some(origin),
            ..self
        }
    }

    /// The window's rectangle with the pointer at `at`: moved by how far the pointer went, or
    /// resized from the edges it grabbed.
    pub fn rect_at(&self, origin: LogicalRect, at: LogicalPoint) -> LogicalRect {
        let (dx, dy) = (
            at.x.saturating_sub(self.start.x),
            at.y.saturating_sub(self.start.y),
        );
        match self.spec {
            GrabSpec::Move { .. } => LogicalRect::new(
                LogicalPoint {
                    x: origin.origin.x.saturating_add(dx),
                    y: origin.origin.y.saturating_add(dy),
                },
                origin.size,
            ),
            GrabSpec::Resize { edges, .. } => resized(origin, edges, (dx, dy)),
        }
    }
}

/// Which sides of the rectangle `edges` moves: (left, right, top, bottom).
fn sides(edges: ResizeEdges) -> (bool, bool, bool, bool) {
    match edges {
        ResizeEdges::Top => (false, false, true, false),
        ResizeEdges::Bottom => (false, false, false, true),
        ResizeEdges::Left => (true, false, false, false),
        ResizeEdges::Right => (false, true, false, false),
        ResizeEdges::TopLeft => (true, false, true, false),
        ResizeEdges::TopRight => (false, true, true, false),
        ResizeEdges::BottomLeft => (true, false, false, true),
        ResizeEdges::BottomRight => (false, true, false, true),
    }
}

/// `origin` with the sides `edges` names moved by `by`; a side never crosses the opposite one.
fn resized(origin: LogicalRect, edges: ResizeEdges, by: (i32, i32)) -> LogicalRect {
    let (left, right, top, bottom) = sides(edges);
    let (x, w) = span(origin.origin.x, origin.size.w, by.0, left, right);
    let (y, h) = span(origin.origin.y, origin.size.h, by.1, top, bottom);
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

/// One axis: the start moved when the near side moves, the length grown or shrunk by the sides
/// that move.
fn span(start: i32, length: u32, by: i32, near: bool, far: bool) -> (i32, u32) {
    let end = i64::from(start) + i64::from(length);
    let new_start = i64::from(start) + if near { i64::from(by) } else { 0 };
    let new_end = end + if far { i64::from(by) } else { 0 };
    let new_start = new_start.min(new_end - i64::from(SMALLEST));
    let clamp = |v: i64| i32::try_from(v).unwrap_or(if v < 0 { i32::MIN } else { i32::MAX });
    (
        clamp(new_start),
        u32::try_from(new_end - new_start).unwrap_or(SMALLEST),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::InputSerial;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    fn point(x: i32, y: i32) -> LogicalPoint {
        LogicalPoint { x, y }
    }

    fn resize(edges: ResizeEdges) -> Drag {
        Drag::begin(
            GrabSpec::Resize {
                window: WindowId(1),
                serial: InputSerial(1),
                edges,
            },
            point(100, 100),
        )
    }

    #[test]
    fn a_move_follows_the_pointer_and_keeps_the_size() {
        let drag = Drag::begin(
            GrabSpec::Move {
                window: WindowId(1),
                serial: InputSerial(1),
            },
            point(100, 100),
        );
        assert_eq!(
            drag.rect_at(rect(10, 20, 300, 200), point(190, 140)),
            rect(100, 60, 300, 200)
        );
    }

    #[test]
    fn a_resize_moves_only_the_sides_it_grabbed() {
        let origin = rect(10, 20, 300, 200);
        let rows = [
            (ResizeEdges::Right, rect(10, 20, 340, 200)),
            (ResizeEdges::Bottom, rect(10, 20, 300, 230)),
            (ResizeEdges::Left, rect(50, 20, 260, 200)),
            (ResizeEdges::Top, rect(10, 50, 300, 170)),
            (ResizeEdges::BottomRight, rect(10, 20, 340, 230)),
            (ResizeEdges::TopLeft, rect(50, 50, 260, 170)),
        ];
        for (edges, want) in rows {
            assert_eq!(
                resize(edges).rect_at(origin, point(140, 130)),
                want,
                "{edges:?}"
            );
        }
    }

    #[test]
    fn a_resize_never_turns_the_window_inside_out() {
        let origin = rect(10, 20, 300, 200);
        let squeezed = resize(ResizeEdges::Left).rect_at(origin, point(1000, 100));
        assert_eq!(squeezed, rect(309, 20, 1, 200));
    }
}
