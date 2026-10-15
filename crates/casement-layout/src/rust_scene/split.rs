//! A Split View Space: its two windows side by side over the whole output with a divider
//! between them. The row is Taffy's (through the bridge); this file decides the widths.

use crate::taffy_bridge::{SplitRects, split_rects};
use crate::{
    Arrangement, BackdropTarget, ConfigureNeed, DividerTarget, LayoutReport, OutputSpec, SpaceSpec,
    WindowSpec, WindowTarget,
};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::space::SplitPair;
use casement_core::window::MaxSize;
use ds_core::vocab::Fraction;

/// The width of the strip between the halves, in logical pixels.
pub(super) const DIVIDER_WIDTH: u32 = 8;

/// Adds the targets of `pair`'s two windows, the divider strip between them and the backdrop
/// that shows in the strip and round a window centred in its half (its maximum size is
/// narrower than the half). Both windows are asked for their size (the half's, or their maximum), as a full-screen window
/// is. A pair with a window the Space does not list gets nothing.
pub(super) fn arrange_split(
    output: &OutputSpec,
    space: &SpaceSpec,
    pair: SplitPair,
    into: &mut Arrangement,
) {
    let find = |id| space.windows.iter().find(|spec| spec.window == id);
    let (Some(leading), Some(trailing)) = (find(pair.leading), find(pair.trailing)) else {
        return;
    };
    let area = output.full;
    let gap = DIVIDER_WIDTH.min(area.size.w);
    let (lead_w, _) = split_widths(
        area.size.w - gap,
        pair.divider,
        leading.limits.min.w,
        trailing.limits.min.w,
    );
    let rects = split_rects(area, lead_w, gap).unwrap_or_else(|| by_arithmetic(area, lead_w, gap));
    into.dividers.push(DividerTarget {
        space: space.space.clone(),
        output: output.output.clone(),
        leading: pair.leading,
        trailing: pair.trailing,
        strip: rects.divider,
        span: area,
        min_leading: leading.limits.min.w,
        min_trailing: trailing.limits.min.w,
    });
    for (spec, half) in [(leading, rects.leading), (trailing, rects.trailing)] {
        let (target, report) = target(spec, output, within_max(half, spec.limits.max));
        into.windows.push(target);
        into.reports.extend(report);
    }
    into.backdrops.push(BackdropTarget {
        space: space.space.clone(),
        output: output.output.clone(),
    });
}

/// The rectangle a window takes in `half`: the half itself, or when its maximum size is
/// smaller on a side, that size centred in the half (as the Mac centres it on a backdrop).
/// A zero side of the maximum means none.
pub(super) fn within_max(half: LogicalRect, max: MaxSize) -> LogicalRect {
    let MaxSize::Bounded(max) = max else {
        return half;
    };
    let fit = |room: u32, most: u32| match most {
        0 => room,
        most => room.min(most),
    };
    let size = LogicalSize {
        w: fit(half.size.w, max.w),
        h: fit(half.size.h, max.h),
    };
    let beside = |from: i32, room: u32, used: u32| {
        let x = i64::from(from) + i64::from((room - used) / 2);
        i32::try_from(x).unwrap_or(i32::MAX)
    };
    LogicalRect::new(
        LogicalPoint {
            x: beside(half.origin.x, half.size.w, size.w),
            y: beside(half.origin.y, half.size.h, size.h),
        },
        size,
    )
}

/// The widths of the two halves out of `room` (the width without the divider): the leading one
/// takes `share` of it, held so each half keeps its window's minimum width. When the minimums
/// cannot both be met the share stands and the caller reports the window that falls short.
pub(super) fn split_widths(
    room: u32,
    share: Fraction,
    min_lead: u32,
    min_trail: u32,
) -> (u32, u32) {
    let want = u64::from(room) * u64::from(share.0.min(1000)) / 1000;
    let want = u32::try_from(want).unwrap_or(room);
    let highest = room.saturating_sub(min_trail);
    let lead = match min_lead <= highest {
        true => want.clamp(min_lead, highest),
        false => want,
    };
    (lead, room - lead)
}

/// The row without Taffy: what the bridge returns when it refuses the tree.
fn by_arithmetic(area: LogicalRect, lead_w: u32, gap: u32) -> SplitRects {
    let part = |from: u32, w: u32| {
        let x = i64::from(area.origin.x) + i64::from(from);
        LogicalRect::new(
            LogicalPoint {
                x: i32::try_from(x).unwrap_or(i32::MAX),
                y: area.origin.y,
            },
            LogicalSize { w, h: area.size.h },
        )
    };
    SplitRects {
        leading: part(0, lead_w),
        divider: part(lead_w, gap),
        trailing: part(lead_w + gap, area.size.w.saturating_sub(lead_w + gap)),
    }
}

/// One half's target: always a resize, and a report when the window's minimum is not met.
fn target(
    spec: &WindowSpec,
    output: &OutputSpec,
    rect: LogicalRect,
) -> (WindowTarget, Option<LayoutReport>) {
    let min = spec.limits.min;
    let unmet = rect.size.w < min.w || rect.size.h < min.h;
    let report = unmet.then_some(LayoutReport::MinUnmet {
        window: spec.window,
        min,
        got: rect.size,
    });
    let target = WindowTarget {
        window: spec.window,
        output: output.output.clone(),
        rect,
        size: ConfigureNeed::Resize(rect.size),
    };
    (target, report)
}
