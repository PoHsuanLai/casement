//! The Mac's placement of new windows: the first centred in the usable area, each later one a
//! cascade step down and right of the one before. A cascade that would leave the area starts
//! over at the centre, so a new window is always inside.

use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};

/// Where the `nth` new window (0 first) of `size` goes in `usable`, cascading by `step`.
/// `size` is already held to `usable`; the result is inside it.
pub(crate) fn cascade_rect(
    usable: LogicalRect,
    size: LogicalSize,
    nth: u32,
    step: LogicalPoint,
) -> LogicalRect {
    let centre = LogicalPoint {
        x: centred(usable.origin.x, usable.size.w, size.w),
        y: centred(usable.origin.y, usable.size.h, size.h),
    };
    let room = |at: i32, step: i32, from: i32, extent: u32, own: u32| {
        steps_inside(at, step, from, extent, own)
    };
    let fit = room(centre.x, step.x, usable.origin.x, usable.size.w, size.w).min(room(
        centre.y,
        step.y,
        usable.origin.y,
        usable.size.h,
        size.h,
    ));
    let turn = u64::from(nth) % fit.saturating_add(1);
    LogicalRect::new(
        LogicalPoint {
            x: moved(centre.x, step.x, turn),
            y: moved(centre.y, step.y, turn),
        },
        size,
    )
}

/// The position that centres `own` in the span of `extent` starting at `from` (an odd
/// leftover goes to the far side).
fn centred(from: i32, extent: u32, own: u32) -> i32 {
    to_i32(i64::from(from) + i64::from(extent.saturating_sub(own) / 2))
}

/// How many whole `step`s a span of `own` at `at` can take and stay inside the `extent` that
/// starts at `from` (unbounded when the step is zero).
fn steps_inside(at: i32, step: i32, from: i32, extent: u32, own: u32) -> u64 {
    let end = i64::from(from) + i64::from(extent);
    let room = match step.signum() {
        1 => end - (i64::from(at) + i64::from(own)),
        -1 => i64::from(at) - i64::from(from),
        _ => return u64::MAX,
    };
    u64::try_from(room.max(0) / i64::from(step.unsigned_abs())).unwrap_or(0)
}

/// `at` moved by `turn` steps.
fn moved(at: i32, step: i32, turn: u64) -> i32 {
    let turn = i64::try_from(turn).unwrap_or(i64::MAX);
    to_i32(i64::from(at).saturating_add(i64::from(step).saturating_mul(turn)))
}

/// `value` held to `i32`'s range.
fn to_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    fn step(x: i32, y: i32) -> LogicalPoint {
        LogicalPoint { x, y }
    }

    /// A name, the usable area, the window size, the cascade step, then each `nth` and where
    /// it goes.
    type Case = (
        &'static str,
        LogicalRect,
        LogicalSize,
        LogicalPoint,
        Vec<(u32, LogicalRect)>,
    );

    #[test]
    fn the_cascade_table() {
        let size = |w, h| LogicalSize { w, h };
        let cases: Vec<Case> = vec![
            (
                "centre, then one step each",
                rect(0, 24, 1920, 1056),
                size(800, 600),
                step(28, 28),
                vec![
                    (0, rect(560, 252, 800, 600)),
                    (1, rect(588, 280, 800, 600)),
                    (8, rect(784, 476, 800, 600)),
                ],
            ),
            (
                "a ninth step would leave the area: start over at the centre",
                rect(0, 24, 1920, 1056),
                size(800, 600),
                step(28, 28),
                vec![
                    (9, rect(560, 252, 800, 600)),
                    (10, rect(588, 280, 800, 600)),
                ],
            ),
            (
                "an odd leftover puts the extra pixel on the far side",
                rect(0, 0, 101, 101),
                size(50, 50),
                step(0, 0),
                vec![(0, rect(25, 25, 50, 50)), (7, rect(25, 25, 50, 50))],
            ),
            (
                "a window as large as the area never moves",
                rect(10, 20, 640, 480),
                size(640, 480),
                step(28, 28),
                vec![(0, rect(10, 20, 640, 480)), (3, rect(10, 20, 640, 480))],
            ),
            (
                "an upward step walks up and left, then wraps",
                rect(0, 0, 400, 400),
                size(200, 200),
                step(-50, -50),
                vec![
                    (0, rect(100, 100, 200, 200)),
                    (2, rect(0, 0, 200, 200)),
                    (3, rect(100, 100, 200, 200)),
                ],
            ),
            (
                "the tighter axis sets the cycle",
                rect(0, 0, 1000, 300),
                size(200, 200),
                step(10, 10),
                vec![(5, rect(450, 100, 200, 200)), (6, rect(400, 50, 200, 200))],
            ),
        ];
        for (name, usable, window, by, rows) in cases {
            for (nth, want) in rows {
                assert_eq!(cascade_rect(usable, window, nth, by), want, "{name}: {nth}");
            }
        }
    }
}
