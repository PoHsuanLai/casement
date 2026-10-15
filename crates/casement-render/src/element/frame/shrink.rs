//! A frame drawn at its own size into a smaller rectangle: Mission Control shows a window and
//! its frame (titlebar, lights, title, shadow) shrunk together, as one picture. The rasters are
//! the full-size ones, so nothing is made again while the windows fly; only the rectangles they
//! are drawn into shrink. Pure whole-number maths, so a table test pins it.

use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};

/// The factor from a window's own content size to the size it is drawn at, as the width ratio
/// (the layout shrinks both sides by one factor; a pixel of rounding in the height is not
/// followed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Shrink {
    full: u32,
    drawn: u32,
}

impl Shrink {
    /// The shrink of content of `full` size drawn at `drawn`. A window with no width, or one
    /// drawn at least as wide as it is, is not shrunk.
    pub(crate) fn of(full: LogicalSize, drawn: LogicalSize) -> Shrink {
        match full.w {
            0 => Shrink::NONE,
            _ => Shrink {
                full: full.w,
                drawn: drawn.w.min(full.w),
            },
        }
    }

    /// No shrink.
    pub(crate) const NONE: Shrink = Shrink { full: 1, drawn: 1 };

    /// Whether this is no shrink at all.
    pub(crate) fn is_none(self) -> bool {
        self.full == self.drawn
    }

    /// `length` logical pixels shrunk, to the nearest pixel.
    pub(crate) fn length(self, length: i64) -> i64 {
        let (full, drawn) = (i64::from(self.full), i64::from(self.drawn));
        (length * drawn * 2 + length.signum() * full) / (2 * full)
    }

    /// `rect` shrunk about its parent's top-left corner (its origin is relative to it): both
    /// corners are shrunk and rounded, so rectangles that touch still touch.
    pub(crate) fn rect(self, rect: LogicalRect) -> LogicalRect {
        let (x0, y0) = (i64::from(rect.origin.x), i64::from(rect.origin.y));
        let (x1, y1) = (x0 + i64::from(rect.size.w), y0 + i64::from(rect.size.h));
        let clamp = |v: i64| i32::try_from(v).unwrap_or(i32::MAX);
        let side = |v: i64| u32::try_from(v.max(0)).unwrap_or(u32::MAX);
        let (sx0, sy0) = (self.length(x0), self.length(y0));
        LogicalRect::new(
            LogicalPoint {
                x: clamp(sx0),
                y: clamp(sy0),
            },
            LogicalSize {
                w: side(self.length(x1) - sx0),
                h: side(self.length(y1) - sy0),
            },
        )
    }

    /// `rect`, relative to `corner`, shrunk about it: the place on the output.
    pub(crate) fn about(self, corner: LogicalPoint, rect: LogicalRect) -> LogicalRect {
        let shrunk = self.rect(rect);
        LogicalRect::new(
            LogicalPoint {
                x: corner.x.saturating_add(shrunk.origin.x),
                y: corner.y.saturating_add(shrunk.origin.y),
            },
            shrunk.size,
        )
    }

    /// A titlebar's height shrunk (at least one pixel for a titlebar that has any).
    pub(crate) fn bar(self, titlebar: u32) -> u32 {
        let shrunk = u32::try_from(self.length(i64::from(titlebar))).unwrap_or(0);
        match titlebar {
            0 => 0,
            _ => shrunk.max(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(w: u32, h: u32) -> LogicalSize {
        LogicalSize { w, h }
    }

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, size(w, h))
    }

    #[test]
    fn lengths_shrink_to_the_nearest_pixel() {
        // name, full width, drawn width, length, want
        let cases = [
            ("whole", 800, 800, 28, 28),
            ("half", 800, 400, 28, 14),
            ("half, odd", 800, 400, 27, 14),
            ("a third", 900, 300, 28, 9),
            ("negative", 800, 400, -80, -40),
            ("zero", 800, 400, 0, 0),
        ];
        for (name, full, drawn, length, want) in cases {
            let shrink = Shrink::of(size(full, 1), size(drawn, 1));
            assert_eq!(shrink.length(length), want, "{name}");
        }
    }

    #[test]
    fn no_width_or_a_bigger_drawing_is_no_shrink() {
        assert!(Shrink::of(size(0, 10), size(50, 10)).is_none());
        assert!(Shrink::of(size(400, 10), size(500, 10)).is_none());
        assert!(Shrink::of(size(400, 10), size(400, 10)).is_none());
        assert!(!Shrink::of(size(400, 10), size(399, 10)).is_none());
    }

    #[test]
    fn touching_rectangles_still_touch_after_a_shrink() {
        let shrink = Shrink::of(size(1000, 1), size(333, 1));
        let (left, right) = (rect(-80, 0, 150, 40), rect(70, 0, 200, 40));
        let (a, b) = (shrink.rect(left), shrink.rect(right));
        assert_eq!(
            i64::from(a.origin.x) + i64::from(a.size.w),
            i64::from(b.origin.x)
        );
    }

    #[test]
    fn a_rectangle_is_placed_about_the_corner() {
        let shrink = Shrink::of(size(400, 1), size(200, 1));
        let got = shrink.about(LogicalPoint { x: 100, y: 50 }, rect(-80, -80, 160, 160));
        assert_eq!(got, rect(60, 10, 80, 80));
    }

    #[test]
    fn a_titlebar_keeps_a_pixel() {
        let shrink = Shrink::of(size(1000, 1), size(10, 1));
        assert_eq!(shrink.bar(28), 1);
        assert_eq!(shrink.bar(0), 0);
        assert_eq!(Shrink::NONE.bar(28), 28);
    }
}
