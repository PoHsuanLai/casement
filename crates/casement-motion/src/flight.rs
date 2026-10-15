//! A minimise flight's two ends and how far along a window's pose is between them.

use crate::pose::Pose;
use casement_core::geometry::LogicalRect;

/// The rectangle a window flies between and the dock tile it flies to or from: the window's
/// rectangle when it left (or where it will be when it is back), and the tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FlightEnds {
    /// The window's end.
    pub(crate) window: LogicalRect,
    /// The tile's end.
    pub(crate) tile: LogicalRect,
}

impl FlightEnds {
    /// How far `pose` is from the window's end to the tile's, 0 to 1, read off the channel
    /// (left, top, width, height) the two ends differ most in; 0 when they do not differ.
    pub(crate) fn progress(&self, pose: Pose) -> f64 {
        let (w, t) = (self.window, self.tile);
        let channels = [
            (f64::from(w.origin.x), f64::from(t.origin.x), pose.x),
            (f64::from(w.origin.y), f64::from(t.origin.y), pose.y),
            (f64::from(w.size.w), f64::from(t.size.w), pose.w),
            (f64::from(w.size.h), f64::from(t.size.h), pose.h),
        ];
        let widest =
            channels
                .into_iter()
                .reduce(|a, b| match (b.1 - b.0).abs() > (a.1 - a.0).abs() {
                    true => b,
                    false => a,
                });
        match widest {
            Some((from, to, at)) if (to - from).abs() > f64::EPSILON => {
                ((at - from) / (to - from)).clamp(0.0, 1.0)
            }
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalPoint, LogicalSize};

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    fn pose(x: f64, y: f64, w: f64, h: f64) -> Pose {
        Pose {
            x,
            y,
            w,
            h,
            opacity: 1.0,
        }
    }

    #[test]
    fn progress_reads_the_widest_channel_between_the_ends() {
        let ends = FlightEnds {
            window: rect(200, 200, 800, 600),
            tile: rect(900, 1040, 48, 30),
        };
        // name, pose, progress (y differs most: 840 against 752 for the width)
        let cases = [
            ("at the window", pose(200.0, 200.0, 800.0, 600.0), 0.0),
            ("halfway", pose(550.0, 620.0, 424.0, 315.0), 0.5),
            ("at the tile", pose(900.0, 1040.0, 48.0, 30.0), 1.0),
            ("past the tile", pose(900.0, 1200.0, 48.0, 30.0), 1.0),
            ("before the window", pose(200.0, 100.0, 800.0, 600.0), 0.0),
        ];
        for (name, pose, want) in cases {
            let got = ends.progress(pose);
            assert!((got - want).abs() < 1e-9, "{name}: {got}");
        }
    }

    #[test]
    fn identical_ends_have_no_progress() {
        let same = rect(10, 10, 100, 100);
        let ends = FlightEnds {
            window: same,
            tile: same,
        };
        assert_eq!(ends.progress(pose(50.0, 50.0, 100.0, 100.0)), 0.0);
    }
}
