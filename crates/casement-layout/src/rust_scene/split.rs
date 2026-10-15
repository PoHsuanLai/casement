//! A Split View Space: its two windows side by side over the whole output with a divider
//! between them. The row is Taffy's (through the bridge); this file decides the widths.

use crate::taffy_bridge::{SplitRects, split_rects};
use crate::{
    Arrangement, ConfigureNeed, LayoutReport, OutputSpec, SpaceSpec, WindowSpec, WindowTarget,
};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::space::SplitPair;
use ds_core::vocab::Fraction;

/// The width of the strip between the halves, in logical pixels.
pub(super) const DIVIDER_WIDTH: u32 = 8;

/// Adds the targets of `pair`'s two windows. Both are asked for their half's size, as a
/// full-screen window is. A pair with a window the Space does not list gets nothing.
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
    for (spec, rect) in [(leading, rects.leading), (trailing, rects.trailing)] {
        let (target, report) = target(spec, output, rect);
        into.windows.push(target);
        into.reports.extend(report);
    }
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
