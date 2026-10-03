//! Where a window is drawn, in the units its legs move in: logical pixels and opacity 0 to 1.

use casement_core::geometry::LogicalRect;

/// A window's rectangle and opacity as floats (the legs' side of the quantising step).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Pose {
    /// The left edge.
    pub(crate) x: f64,
    /// The top edge.
    pub(crate) y: f64,
    /// The width.
    pub(crate) w: f64,
    /// The height.
    pub(crate) h: f64,
    /// The opacity, 0 to 1.
    pub(crate) opacity: f64,
}

/// How much smaller than its place a window is while it arrives or leaves.
const TUCK: f64 = 0.92;

impl Pose {
    /// A window fully shown at `rect`.
    pub(crate) fn shown(rect: LogicalRect) -> Pose {
        Pose {
            x: f64::from(rect.origin.x),
            y: f64::from(rect.origin.y),
            w: f64::from(rect.size.w),
            h: f64::from(rect.size.h),
            opacity: 1.0,
        }
    }

    /// The same pose, invisible.
    pub(crate) fn faded(self) -> Pose {
        Pose {
            opacity: 0.0,
            ..self
        }
    }

    /// The same pose a little smaller about its centre.
    pub(crate) fn tucked(self) -> Pose {
        Pose {
            x: self.x + self.w * (1.0 - TUCK) / 2.0,
            y: self.y + self.h * (1.0 - TUCK) / 2.0,
            w: self.w * TUCK,
            h: self.h * TUCK,
            ..self
        }
    }

    /// The five channels in leg order.
    pub(crate) fn channels(self) -> [f64; 5] {
        [self.x, self.y, self.w, self.h, self.opacity]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalPoint, LogicalSize};

    #[test]
    fn a_tucked_pose_keeps_its_centre() {
        let rect = LogicalRect::new(
            LogicalPoint { x: 100, y: 50 },
            LogicalSize { w: 400, h: 200 },
        );
        let pose = Pose::shown(rect).tucked();
        assert_eq!((pose.w, pose.h), (368.0, 184.0));
        assert_eq!(
            (pose.x + pose.w / 2.0, pose.y + pose.h / 2.0),
            (300.0, 150.0)
        );
        assert_eq!(pose.opacity, 1.0);
        assert_eq!(Pose::shown(rect).faded().opacity, 0.0);
    }
}
