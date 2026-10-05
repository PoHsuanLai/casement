//! An interactive move or resize the person is doing: where the pointer was when it began, the
//! window's rectangle then, and what a pointer position makes of that rectangle. The seat
//! holds the one grab there can be; the core loop starts it (`Effect::StartGrab`), feeds it
//! the pointer and ends it at a button release.

use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::input::ButtonCode;
use casement_core::window::{MaxSize, SizeLimits};
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
    /// The pointer button the grab began with: it ends when that button is released, and not
    /// when another is (xdg-shell: the grab follows the press its serial answers). None when
    /// the press that began it is not remembered.
    pub button: Option<ButtonCode>,
}

impl Drag {
    /// A grab of `spec` that began with the pointer at `start`, holding `button`.
    pub fn begin(spec: GrabSpec, start: LogicalPoint, button: Option<ButtonCode>) -> Drag {
        Drag {
            spec,
            start,
            origin: None,
            button,
        }
    }

    /// Whether releasing `released` ends the grab: its own button does, and any button does
    /// when the grab does not know which one began it.
    pub fn ends_with(&self, released: ButtonCode) -> bool {
        self.button.is_none_or(|held| held == released)
    }

    /// The serial of the press that began it.
    pub fn serial(&self) -> casement_core::InputSerial {
        match self.spec {
            GrabSpec::Move { serial, .. } | GrabSpec::Resize { serial, .. } => serial,
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
    /// resized from the edges it grabbed, within `limits` (the side opposite the one grabbed
    /// stays where it is when a limit holds the size).
    pub fn rect_at(
        &self,
        origin: LogicalRect,
        at: LogicalPoint,
        limits: SizeLimits,
    ) -> LogicalRect {
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
            GrabSpec::Resize { edges, .. } => resized(origin, edges, (dx, dy), limits),
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

/// `origin` with the sides `edges` names moved by `by`, its size held to `limits`.
fn resized(
    origin: LogicalRect,
    edges: ResizeEdges,
    by: (i32, i32),
    limits: SizeLimits,
) -> LogicalRect {
    let (left, right, top, bottom) = sides(edges);
    let max = |pick: fn(LogicalSize) -> u32| match limits.max {
        MaxSize::Unbounded => None,
        MaxSize::Bounded(size) => Some(pick(size)),
    };
    let across = Span {
        near: left,
        far: right,
        least: limits.min.w,
        most: max(|size| size.w),
    };
    let down = Span {
        near: top,
        far: bottom,
        least: limits.min.h,
        most: max(|size| size.h),
    };
    let (x, w) = across.of(origin.origin.x, origin.size.w, by.0);
    let (y, h) = down.of(origin.origin.y, origin.size.h, by.1);
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

/// What moves on one axis and how far the window may shrink or grow along it.
#[derive(Debug, Clone, Copy)]
struct Span {
    /// Whether the near side (left, top) moves.
    near: bool,
    /// Whether the far side (right, bottom) moves.
    far: bool,
    /// The least length (0: none).
    least: u32,
    /// The most length.
    most: Option<u32>,
}

impl Span {
    /// The start and length of the window's extent on this axis, which began at `start` with
    /// `length`, with the pointer `by` further on: the sides that move follow it; the length is
    /// held between the limits (never below one); the far side stays when the near one moves.
    fn of(self, start: i32, length: u32, by: i32) -> (i32, u32) {
        if !self.near && !self.far {
            return (start, length);
        }
        let end = i64::from(start) + i64::from(length);
        let grown = i64::from(length) + if self.far { i64::from(by) } else { 0 }
            - if self.near { i64::from(by) } else { 0 };
        let floor = i64::from(self.least.max(SMALLEST));
        let ceiling = self.most.map_or(i64::MAX, i64::from).max(floor);
        let held = grown.clamp(floor, ceiling);
        let new_start = if self.near {
            end - held
        } else {
            i64::from(start)
        };
        let clamp = |v: i64| i32::try_from(v).unwrap_or(if v < 0 { i32::MIN } else { i32::MAX });
        (clamp(new_start), u32::try_from(held).unwrap_or(SMALLEST))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::InputSerial;

    const NONE: SizeLimits = SizeLimits {
        min: LogicalSize { w: 0, h: 0 },
        max: MaxSize::Unbounded,
    };

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
            None,
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
            None,
        );
        assert_eq!(
            drag.rect_at(rect(10, 20, 300, 200), point(190, 140), NONE),
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
                resize(edges).rect_at(origin, point(140, 130), NONE),
                want,
                "{edges:?}"
            );
        }
    }

    #[test]
    fn a_resize_never_turns_the_window_inside_out() {
        let origin = rect(10, 20, 300, 200);
        let squeezed = resize(ResizeEdges::Left).rect_at(origin, point(1000, 100), NONE);
        assert_eq!(squeezed, rect(309, 20, 1, 200));
    }

    #[test]
    fn limits_hold_the_size_and_the_far_side_stays() {
        let origin = rect(10, 20, 300, 200);
        let limits = SizeLimits {
            min: LogicalSize { w: 250, h: 150 },
            max: MaxSize::Bounded(LogicalSize { w: 400, h: 260 }),
        };
        let rows = [
            // (edges, pointer, wanted)
            (ResizeEdges::Left, point(200, 100), rect(60, 20, 250, 200)),
            (ResizeEdges::Left, point(-100, 100), rect(-90, 20, 400, 200)),
            (
                ResizeEdges::TopLeft,
                point(300, 300),
                rect(60, 70, 250, 150),
            ),
            (
                ResizeEdges::BottomRight,
                point(0, 0),
                rect(10, 20, 250, 150),
            ),
            (
                ResizeEdges::BottomRight,
                point(900, 900),
                rect(10, 20, 400, 260),
            ),
            // An axis that is not grabbed keeps its length even outside the limits.
            (ResizeEdges::Right, point(100, 400), rect(10, 20, 300, 200)),
        ];
        for (edges, at, want) in rows {
            assert_eq!(
                resize(edges).rect_at(origin, at, limits),
                want,
                "{edges:?} to {at:?}"
            );
        }
    }
}
