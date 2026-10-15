//! Mission Control's spread (P3-Z freezes `rust_scene/spread.rs`; the recipe is in
//! `RustScene`'s module doc and 21-FREEZE-AMENDMENTS A21). Written now so the P3 lane inherits
//! the properties SP8 proved.

use crate::support;

use casement_core::geometry::LogicalRect;
use casement_layout::{
    LayoutReport, OutputSpec, OverviewKind, RustScene, SceneLayout, SceneMode, SceneSpec, SlotKind,
    WindowPlacement,
};
use proptest::prelude::*;
use support::scene::{ANY_SIZE, knobs, one_desktop, rect, space_id, window};

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

fn mission_control(spec: SceneSpec) -> SceneSpec {
    SceneSpec {
        outputs: spec
            .outputs
            .into_iter()
            .map(|o| OutputSpec {
                mode: SceneMode::Overview {
                    shown: space_id("s-1"),
                    kind: OverviewKind::MissionControl,
                },
                ..o
            })
            .collect(),
    }
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
        let spec = mission_control(one_desktop(usable, windows));
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
            for b in &got.overview[i + 1..] {
                prop_assert!(overflow || gap(&a.rect, &b.rect) >= 20, "{:?} and {:?} closer than the gap", a.window, b.window);
            }
        }
        prop_assert_eq!(RustScene.arrange(&spec, &knobs()), got);
    }
}
