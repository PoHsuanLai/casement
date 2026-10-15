//! Where the app's icon sits under a Mission Control slot: centred, its middle a little below
//! the slot's bottom edge (as the Mac does), 64 logical pixels across on a window shown whole,
//! shrinking with the slot like the frame does, never under 32.

use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};

/// One part in a million: the spread's scale unit.
const WHOLE: i64 = 1_000_000;

/// The icon's edge on a window shown at its own size, in logical pixels.
const REST: i64 = 64;

/// The smallest edge, in logical pixels.
const LEAST: i64 = 32;

/// How far the icon's middle is below the slot's bottom edge, in logical pixels.
const DROP: i64 = 6;

/// The icon for `slot`, a window drawn at `scale` (millionths) of its size, held inside `usable`
/// (an icon that would hang below the area is lifted until it fits).
pub(super) fn icon_rect(slot: LogicalRect, scale: i64, usable: LogicalRect) -> LogicalRect {
    let edge = (REST * scale / WHOLE).clamp(LEAST, REST);
    let left = i64::from(slot.origin.x) + (i64::from(slot.size.w) - edge) / 2;
    let bottom = i64::from(slot.origin.y) + i64::from(slot.size.h);
    let area_bottom = i64::from(usable.origin.y) + i64::from(usable.size.h);
    let top = (bottom + DROP - edge / 2).min(area_bottom - edge);
    let side = u32::try_from(edge).unwrap_or(0);
    LogicalRect::new(
        LogicalPoint {
            x: whole(left),
            y: whole(top),
        },
        LogicalSize { w: side, h: side },
    )
}

/// `n` as a logical coordinate.
fn whole(n: i64) -> i32 {
    i32::try_from(n).unwrap_or(if n < 0 { i32::MIN } else { i32::MAX })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    #[test]
    fn the_icon_is_centred_under_the_slot_and_sized_with_it() {
        let usable = rect(0, 0, 1920, 1080);
        let slot = rect(100, 200, 400, 300);
        // name, scale (millionths), want (x, y, edge)
        let cases = [
            ("whole", 1_000_000, (268, 474, 64)),
            ("half", 500_000, (284, 490, 32)),
            ("a small slot keeps the least", 100_000, (284, 490, 32)),
            ("never above the rest size", 2_000_000, (268, 474, 64)),
        ];
        for (name, scale, (x, y, edge)) in cases {
            let got = icon_rect(slot, scale, usable);
            assert_eq!(
                (got.origin.x, got.origin.y, got.size.w, got.size.h),
                (x, y, edge, edge),
                "{name}"
            );
        }
    }

    #[test]
    fn an_icon_at_the_bottom_of_the_area_is_lifted_to_fit() {
        let usable = rect(0, 0, 1000, 800);
        let got = icon_rect(rect(100, 600, 300, 200), 1_000_000, usable);
        assert_eq!(got.origin.y + 64, 800);
    }
}
