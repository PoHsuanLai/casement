//! A floating window's target: kept where it is when it can still be grabbed; otherwise its
//! size held to its limits, then to the usable area (an unmet minimum is reported), and its
//! position pulled back the shortest way until it can (the Mac's rule: a window may go off the
//! left, right and bottom, never above the menu bar, while its grip stays on screen).

use crate::{ConfigureNeed, LayoutReport, WindowTarget};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::window::SizeLimits;
use casement_core::{OutputKey, WindowId};

/// Where a floating window at `rect` on `output` goes in `usable`, keeping `grip` of it there.
pub(crate) fn floating_target(
    window: WindowId,
    output: &OutputKey,
    rect: LogicalRect,
    limits: SizeLimits,
    usable: LogicalRect,
    grip: LogicalSize,
) -> (WindowTarget, Option<LayoutReport>) {
    let size = fit_size(rect.size, limits, usable);
    let at = within_reach(rect.origin, size, usable, grip);
    placed(
        window,
        output,
        rect.size,
        LogicalRect::new(at, size),
        limits,
    )
}

/// The target for a window put at `rect` by hand: its size fitted as ever, its position kept
/// where it was put, even partly outside `usable`.
pub(crate) fn placed_target(
    window: WindowId,
    output: &OutputKey,
    rect: LogicalRect,
    limits: SizeLimits,
    usable: LogicalRect,
) -> (WindowTarget, Option<LayoutReport>) {
    let size = fit_size(rect.size, limits, usable);
    placed(
        window,
        output,
        rect.size,
        LogicalRect::new(rect.origin, size),
        limits,
    )
}

/// The target for a window that fills `usable` (a maximise): its size held to its limits, at
/// the area's origin. A fill asks nothing of its own, so the size always goes out as a
/// configure (the wire drops one that repeats what the client was told).
pub(crate) fn filled_target(
    window: WindowId,
    output: &OutputKey,
    limits: SizeLimits,
    usable: LogicalRect,
) -> (WindowTarget, Option<LayoutReport>) {
    let size = fit_size(usable.size, limits, usable);
    placed(
        window,
        output,
        LogicalSize { w: 0, h: 0 },
        LogicalRect::new(usable.origin, size),
        limits,
    )
}

/// The target for a full-screen window: the whole output `full`, whatever the window's limits
/// say (a full-screen window is as large as the output or it is not full screen). Like a fill
/// it asks nothing of its own, so the size always goes out as a configure.
pub(crate) fn full_screen_target(
    window: WindowId,
    output: &OutputKey,
    full: LogicalRect,
) -> WindowTarget {
    WindowTarget {
        window,
        output: output.clone(),
        rect: full,
        size: ConfigureNeed::Resize(full.size),
    }
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
        None => want,
        Some(max) => LogicalSize {
            w: want.w.min(max.w),
            h: want.h.min(max.h),
        },
    };
    LogicalSize {
        w: capped.w.max(limits.min.w).max(1).min(usable.size.w),
        h: capped.h.max(limits.min.h).max(1).min(usable.size.h),
    }
}

/// `at` moved the shortest way that leaves `grip` of a window of `size` inside `usable` (`size`
/// fits it): across, at least the grip's width stays on either side; down, the top stays
/// below the area's top and the grip's height above its bottom.
pub(crate) fn within_reach(
    at: LogicalPoint,
    size: LogicalSize,
    usable: LogicalRect,
    grip: LogicalSize,
) -> LogicalPoint {
    let across = Span::of(usable.origin.x, usable.size.w);
    let down = Span::of(usable.origin.y, usable.size.h);
    LogicalPoint {
        x: across.hold(
            at.x,
            i64::from(grip.w.min(size.w)) - i64::from(size.w),
            grip.w.min(size.w),
        ),
        y: down.hold(at.y, 0, grip.h.min(size.h)),
    }
}

/// One axis of the usable area: where it starts and how far it goes.
#[derive(Debug, Clone, Copy)]
struct Span {
    from: i64,
    extent: i64,
}

impl Span {
    fn of(from: i32, extent: u32) -> Span {
        Span {
            from: i64::from(from),
            extent: i64::from(extent),
        }
    }

    /// `at` held between the span's start moved by `before` (how far the window may stick out
    /// ahead of it) and its end less `kept` (how much must stay inside before the end).
    fn hold(self, at: i32, before: i64, kept: u32) -> i32 {
        let first = self.from + before;
        let last = (self.from + self.extent - i64::from(kept)).max(first);
        let held = i64::from(at).clamp(first, last);
        i32::try_from(held).unwrap_or(i32::MAX)
    }
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
            max: max.map(|(w, h)| size(w, h)),
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
    fn a_window_may_leave_the_area_while_its_grip_stays() {
        let usable = rect(100, 24, 800, 600);
        let grip = size(112, 28);
        let cases = [
            ("inside stays", (150, 100), (150, 100)),
            ("partly off the left stays", (-50, 100), (-50, 100)),
            (
                "past the left: the grip's width stays",
                (-500, 100),
                (-88, 100),
            ),
            ("partly off the right stays", (700, 100), (700, 100)),
            (
                "past the right: the grip's width stays",
                (850, 100),
                (788, 100),
            ),
            ("never above the area's top", (150, -500), (150, 24)),
            ("partly below stays", (150, 500), (150, 500)),
            (
                "past the bottom: the grip's height stays",
                (150, 5000),
                (150, 596),
            ),
            ("far off both", (i32::MIN, i32::MAX), (-88, 596)),
        ];
        for (name, (x, y), (want_x, want_y)) in cases {
            let got = within_reach(LogicalPoint { x, y }, size(300, 200), usable, grip);
            assert_eq!((got.x, got.y), (want_x, want_y), "{name}");
        }
    }

    #[test]
    fn a_window_smaller_than_the_grip_stays_whole_inside() {
        let usable = rect(100, 24, 800, 600);
        let got = within_reach(
            LogicalPoint { x: -500, y: 5000 },
            size(80, 20),
            usable,
            size(112, 28),
        );
        assert_eq!((got.x, got.y), (100, 604));
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
            size(112, 28),
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
            size(112, 28),
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
