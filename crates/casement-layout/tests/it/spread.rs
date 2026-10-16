//! Mission Control's spread (P3-Z freezes `rust_scene/spread.rs`; the recipe is in
//! `RustScene`'s module doc and 21-FREEZE-AMENDMENTS A21). Written now so the P3 lane inherits
//! the properties SP8 proved.

use crate::support;

use casement_core::WindowId;
use casement_core::geometry::LogicalRect;
use casement_layout::{LayoutReport, RustScene, SceneLayout, SlotKind, WindowPlacement};
use ds_core::vocab::Fraction;
use proptest::prelude::*;
use support::scene::{ANY_SIZE, knobs, one_desktop, rect, window};

/// The distance between two rectangles' nearest edges (0 when they touch or overlap).
fn gap(a: &LogicalRect, b: &LogicalRect) -> i64 {
    let (ax0, ax1) = (
        i64::from(a.origin.x),
        i64::from(a.origin.x) + i64::from(a.size.w),
    );
    let (bx0, bx1) = (
        i64::from(b.origin.x),
        i64::from(b.origin.x) + i64::from(b.size.w),
    );
    let (ay0, ay1) = (
        i64::from(a.origin.y),
        i64::from(a.origin.y) + i64::from(a.size.h),
    );
    let (by0, by1) = (
        i64::from(b.origin.y),
        i64::from(b.origin.y) + i64::from(b.size.h),
    );
    let dx = (bx0 - ax1).max(ax0 - bx1).max(0);
    let dy = (by0 - ay1).max(ay0 - by1).max(0);
    dx.max(dy)
}

proptest! {
    #[test]
    fn the_spread_separates_windows_or_reports_overflow(
        sizes in proptest::collection::vec((320u32..1000, 220u32..720, 0i32..1500, 0i32..800), 1..50),
    ) {
        let usable = rect(0, 24, 1872, 860);
        let windows = sizes
            .iter()
            .enumerate()
            .map(|(n, (w, h, x, y))| window(n as u64 + 1, WindowPlacement::Floating(rect(*x, *y, *w, *h)), ANY_SIZE))
            .collect::<Vec<_>>();
        let spec = one_desktop(usable, windows).in_mission_control();
        let got = RustScene.arrange(&spec, &knobs());
        prop_assert_eq!(got.overview.len(), sizes.len());
        let overflow = got.reports.iter().any(|r| matches!(r, LayoutReport::SpreadOverflow(_)));
        for (i, a) in got.overview.iter().enumerate() {
            prop_assert_eq!(a.kind, SlotKind::Live);
            prop_assert_eq!(a.rect.intersection(&usable), Some(a.rect));
            // The icon is a square of 32 to 64, inside the area, centred under the slot.
            prop_assert_eq!(a.icon.intersection(&usable), Some(a.icon));
            prop_assert!((32..=64).contains(&a.icon.size.w) && a.icon.size.w == a.icon.size.h);
            let (slot_mid, icon_mid) = (
                i64::from(a.rect.origin.x) * 2 + i64::from(a.rect.size.w),
                i64::from(a.icon.origin.x) * 2 + i64::from(a.icon.size.w),
            );
            prop_assert!((slot_mid - icon_mid).abs() <= 1);
            for b in &got.overview {
                // An icon never covers another window's slot.
                prop_assert!(
                    overflow || b.window == a.window || a.icon.intersection(&b.rect).is_none(),
                    "{:?}'s icon covers {:?}", a.window, b.window
                );
            }
            for b in &got.overview[i + 1..] {
                prop_assert!(overflow || gap(&a.rect, &b.rect) >= 20, "{:?} and {:?} closer than the gap", a.window, b.window);
            }
        }
        prop_assert_eq!(RustScene.arrange(&spec, &knobs()), got);
    }
}

/// The fingers part-way up: each window is `openness` thousandths of the way from its desktop
/// rectangle to its slot, at 0 on the desktop and at 1000 in the slot, and the result keeps
/// the Spaces and no slots.
#[test]
fn a_part_open_spread_mixes_each_window_between_desktop_and_slot() {
    let usable = rect(0, 24, 1872, 860);
    let desktop = [rect(100, 100, 600, 400), rect(900, 300, 500, 300)];
    let windows = desktop
        .iter()
        .zip(1u64..)
        .map(|(rect, n)| window(n, WindowPlacement::Floating(*rect), ANY_SIZE))
        .collect();
    let spread = RustScene.arrange(&one_desktop(usable, windows).in_mission_control(), &knobs());
    let slot = |n| {
        spread
            .overview
            .iter()
            .find(|slot| slot.window == WindowId(n))
            .expect("a slot")
            .rect
    };
    let at = |openness| -> Vec<LogicalRect> {
        spread
            .spread_at(Fraction(openness))
            .windows
            .iter()
            .map(|target| target.rect)
            .collect()
    };
    assert_eq!(at(0), desktop.to_vec(), "closed is the desktop");
    assert_eq!(at(1000), vec![slot(1), slot(2)], "open is the slots");
    assert_eq!(at(5000), at(1000), "held to fully open");
    let mid = |from: LogicalRect, to: LogicalRect| LogicalRect {
        origin: casement_core::geometry::LogicalPoint {
            x: from.origin.x + (to.origin.x - from.origin.x) / 2,
            y: from.origin.y + (to.origin.y - from.origin.y) / 2,
        },
        size: casement_core::geometry::LogicalSize {
            w: u32::try_from(
                i64::from(from.size.w) + (i64::from(to.size.w) - i64::from(from.size.w)) / 2,
            )
            .expect("positive"),
            h: u32::try_from(
                i64::from(from.size.h) + (i64::from(to.size.h) - i64::from(from.size.h)) / 2,
            )
            .expect("positive"),
        },
    };
    assert_eq!(
        at(500),
        vec![mid(desktop[0], slot(1)), mid(desktop[1], slot(2))],
        "half open is half way on every edge"
    );
    let half = spread.spread_at(Fraction(500));
    assert_eq!(
        (half.overview.len(), half.spaces),
        (0, spread.spaces.clone())
    );
}
