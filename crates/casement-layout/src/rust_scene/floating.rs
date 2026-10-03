//! A floating window's target: kept where it is when it fits; otherwise its size held to its
//! limits, then to the usable area (an unmet minimum is reported), and its position pushed
//! inside.

use crate::{ConfigureNeed, LayoutReport, WindowTarget};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::window::{MaxSize, SizeLimits};
use casement_core::{OutputKey, WindowId};

/// Where a floating window at `rect` on `output` goes inside `usable`.
pub(crate) fn floating_target(
    window: WindowId,
    output: &OutputKey,
    rect: LogicalRect,
    limits: SizeLimits,
    usable: LogicalRect,
) -> (WindowTarget, Option<LayoutReport>) {
    let size = fit_size(rect.size, limits, usable);
    let at = push_inside(rect.origin, size, usable);
    placed(
        window,
        output,
        rect.size,
        LogicalRect::new(at, size),
        limits,
    )
}

/// The target for a window that asked for `asked` and goes at `rect` (already fitted): a
/// `Resize` when the size changed, and a report when its minimum is not met.
pub(crate) fn placed(
    window: WindowId,
    output: &OutputKey,
    asked: LogicalSize,
    rect: LogicalRect,
    limits: SizeLimits,
) -> (WindowTarget, Option<LayoutReport>) {
    let size = if rect.size == asked {
        ConfigureNeed::Keep
    } else {
        ConfigureNeed::Resize(rect.size)
    };
    let unmet = rect.size.w < limits.min.w || rect.size.h < limits.min.h;
    let report = unmet.then_some(LayoutReport::MinUnmet {
        window,
        min: limits.min,
        got: rect.size,
    });
    let target = WindowTarget {
        window,
        output: output.clone(),
        rect,
        size,
    };
    (target, report)
}

/// `want` held to the window's maximum, raised to its minimum, then held to `usable` (a
/// minimum larger than the area yields to it; the caller reports that), never below one pixel
/// unless the area has none.
pub(crate) fn fit_size(want: LogicalSize, limits: SizeLimits, usable: LogicalRect) -> LogicalSize {
    let capped = match limits.max {
        MaxSize::Unbounded => want,
        MaxSize::Bounded(max) => LogicalSize {
            w: want.w.min(max.w),
            h: want.h.min(max.h),
        },
    };
    LogicalSize {
        w: capped.w.max(limits.min.w).max(1).min(usable.size.w),
        h: capped.h.max(limits.min.h).max(1).min(usable.size.h),
    }
}

/// `at` moved the shortest way that puts a window of `size` inside `usable` (`size` fits it).
pub(crate) fn push_inside(
    at: LogicalPoint,
    size: LogicalSize,
    usable: LogicalRect,
) -> LogicalPoint {
    LogicalPoint {
        x: inside(at.x, size.w, usable.origin.x, usable.size.w),
        y: inside(at.y, size.h, usable.origin.y, usable.size.h),
    }
}

/// `at` held so a span of `own` stays within the `extent` that starts at `from`.
fn inside(at: i32, own: u32, from: i32, extent: u32) -> i32 {
    let last = i64::from(from) + i64::from(extent.saturating_sub(own));
    let held = i64::from(at).clamp(i64::from(from), last);
    i32::try_from(held).unwrap_or(from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    fn size(w: u32, h: u32) -> LogicalSize {
        LogicalSize { w, h }
    }

    fn limits(min: (u32, u32), max: Option<(u32, u32)>) -> SizeLimits {
        SizeLimits {
            min: size(min.0, min.1),
            max: max.map_or(MaxSize::Unbounded, |(w, h)| MaxSize::Bounded(size(w, h))),
        }
    }

    /// A name, the asked size, the limits, the fitted size.
    type SizeCase = (&'static str, LogicalSize, SizeLimits, LogicalSize);

    #[test]
    fn sizes_are_held_to_limits_then_the_area() {
        let usable = rect(0, 24, 1920, 1056);
        let cases: Vec<SizeCase> = vec![
            (
                "unlimited inside",
                size(400, 300),
                limits((0, 0), None),
                size(400, 300),
            ),
            (
                "raised to the minimum",
                size(100, 100),
                limits((200, 150), None),
                size(200, 150),
            ),
            (
                "capped at the maximum",
                size(900, 900),
                limits((0, 0), Some((500, 400))),
                size(500, 400),
            ),
            (
                "a client-side zero becomes one pixel",
                size(0, 0),
                limits((0, 0), None),
                size(1, 1),
            ),
            (
                "held to the area",
                size(3000, 2000),
                limits((0, 0), None),
                size(1920, 1056),
            ),
            (
                "a minimum beyond the area yields to it",
                size(400, 300),
                limits((2500, 100), None),
                size(1920, 300),
            ),
            (
                "a maximum below the minimum: the minimum wins",
                size(400, 300),
                limits((300, 300), Some((200, 200))),
                size(300, 300),
            ),
        ];
        for (name, want, limits, got) in cases {
            assert_eq!(fit_size(want, limits, usable), got, "{name}");
        }
    }

    #[test]
    fn positions_are_pushed_the_shortest_way_inside() {
        let usable = rect(100, 24, 800, 600);
        let cases = [
            ("inside stays", (150, 100), (150, 100)),
            ("left of the area", (-40, 100), (100, 100)),
            ("past the right edge", (880, 100), (800, 100)),
            ("above", (150, -500), (150, 24)),
            ("below", (150, 5000), (150, 424)),
            ("far off both", (i32::MIN, i32::MAX), (100, 424)),
        ];
        for (name, (x, y), (want_x, want_y)) in cases {
            let got = push_inside(LogicalPoint { x, y }, size(100, 200), usable);
            assert_eq!((got.x, got.y), (want_x, want_y), "{name}");
        }
    }

    #[test]
    fn a_resize_and_a_report_follow_the_fit() {
        let usable = rect(0, 24, 1920, 1056);
        let window = WindowId(3);
        let output = OutputKey::of(
            &casement_core::output::EdidFacts {
                make: String::new(),
                model: String::new(),
                serial: casement_core::output::EdidSerial::Unknown,
            },
            &casement_core::OutputName::parse("HEADLESS-1").expect("name"),
        );
        let (kept, none) = floating_target(
            window,
            &output,
            rect(10, 40, 300, 200),
            limits((0, 0), None),
            usable,
        );
        assert_eq!(
            (kept.rect, kept.size, none),
            (rect(10, 40, 300, 200), ConfigureNeed::Keep, None)
        );
        let (squeezed, report) = floating_target(
            window,
            &output,
            rect(0, 24, 400, 2000),
            limits((400, 1200), None),
            usable,
        );
        assert_eq!(squeezed.size, ConfigureNeed::Resize(size(400, 1056)));
        assert_eq!(
            report,
            Some(LayoutReport::MinUnmet {
                window,
                min: size(400, 1200),
                got: size(400, 1056),
            })
        );
    }
}
