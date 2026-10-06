//! Logical (scale-independent) points, sizes and rectangles: what the model and layout speak.

use serde::{Deserialize, Serialize};

/// A point in the global logical space (outputs are placed in it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogicalPoint {
    /// Rightward, in logical pixels.
    pub x: i32,
    /// Downward, in logical pixels.
    pub y: i32,
}

/// A width and a height in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogicalSize {
    /// The width.
    pub w: u32,
    /// The height.
    pub h: u32,
}

impl LogicalSize {
    /// No extent.
    pub const ZERO: LogicalSize = LogicalSize { w: 0, h: 0 };
}

/// An axis-aligned rectangle in logical pixels, half-open: it holds `origin` and stops just
/// before `origin + size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogicalRect {
    /// The top-left corner.
    pub origin: LogicalPoint,
    /// The extent.
    pub size: LogicalSize,
}

impl LogicalRect {
    /// The rectangle at `origin` with `size`.
    pub fn new(origin: LogicalPoint, size: LogicalSize) -> LogicalRect {
        LogicalRect { origin, size }
    }

    /// Whether `p` lies inside (the right and bottom edges are outside).
    pub fn contains(&self, p: LogicalPoint) -> bool {
        let (x, y) = (i64::from(p.x), i64::from(p.y));
        let (left, top) = (i64::from(self.origin.x), i64::from(self.origin.y));
        x >= left
            && y >= top
            && x < left + i64::from(self.size.w)
            && y < top + i64::from(self.size.h)
    }

    /// The part both rectangles cover, or `None` when they share no pixel.
    pub fn intersection(&self, other: &LogicalRect) -> Option<LogicalRect> {
        let left = self.left().max(other.left());
        let top = self.top().max(other.top());
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        (right > left && bottom > top).then(|| LogicalRect {
            origin: LogicalPoint {
                x: clamp_i32(left),
                y: clamp_i32(top),
            },
            size: LogicalSize {
                w: clamp_u32(right - left),
                h: clamp_u32(bottom - top),
            },
        })
    }

    /// The centre, rounded toward the origin on an odd size.
    pub fn centre(&self) -> LogicalPoint {
        LogicalPoint {
            x: clamp_i32(self.left() + i64::from(self.size.w / 2)),
            y: clamp_i32(self.top() + i64::from(self.size.h / 2)),
        }
    }

    /// The number of logical pixels covered.
    pub fn area(&self) -> u64 {
        u64::from(self.size.w) * u64::from(self.size.h)
    }

    fn left(&self) -> i64 {
        i64::from(self.origin.x)
    }

    fn top(&self) -> i64 {
        i64::from(self.origin.y)
    }

    fn right(&self) -> i64 {
        self.left() + i64::from(self.size.w)
    }

    fn bottom(&self) -> i64 {
        self.top() + i64::from(self.size.h)
    }
}

/// `value` held to `i32`'s range.
fn clamp_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

/// `value` held to `u32`'s range.
fn clamp_u32(value: i64) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    #[test]
    fn contains_is_half_open() {
        const CASES: &[(&str, (i32, i32), bool)] = &[
            ("origin", (10, 20), true),
            ("inside", (50, 50), true),
            ("last pixel", (109, 69), true),
            ("right edge", (110, 50), false),
            ("bottom edge", (50, 70), false),
            ("left of it", (9, 50), false),
        ];
        let r = rect(10, 20, 100, 50);
        for (name, (x, y), want) in CASES {
            assert_eq!(r.contains(LogicalPoint { x: *x, y: *y }), *want, "{name}");
        }
    }

    #[test]
    fn intersection_is_the_shared_part_or_nothing() {
        let cases: &[(&str, LogicalRect, Option<LogicalRect>)] = &[
            (
                "overlap",
                rect(50, 40, 100, 100),
                Some(rect(50, 40, 60, 30)),
            ),
            ("inside", rect(20, 30, 10, 10), Some(rect(20, 30, 10, 10))),
            ("touching edge", rect(110, 20, 10, 10), None),
            ("apart", rect(500, 500, 10, 10), None),
        ];
        let r = rect(10, 20, 100, 50);
        for (name, other, want) in cases {
            assert_eq!(r.intersection(other), *want, "{name}");
            assert_eq!(other.intersection(&r), *want, "{name} (swapped)");
        }
    }

    #[test]
    fn centre_and_area() {
        let r = rect(-10, 0, 21, 10);
        assert_eq!(r.centre(), LogicalPoint { x: 0, y: 5 });
        assert_eq!(r.area(), 210);
        assert_eq!(rect(0, 0, u32::MAX, 2).area(), u64::from(u32::MAX) * 2);
    }
}
