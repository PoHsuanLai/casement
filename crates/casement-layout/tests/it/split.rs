//! Split View: the two halves of a Split View Space share the whole output with a divider
//! between them; the divider's share and each window's minimum width decide the widths.

use crate::support::scene::{ANY_SIZE, knobs, one_desktop, rect, size, window};
use casement_core::WindowId;
use casement_core::geometry::LogicalRect;
use casement_core::space::SplitPair;
use casement_core::window::{MaxSize, SizeLimits};
use casement_layout::{
    Arrangement, ConfigureNeed, LayoutReport, RustScene, SceneLayout, SceneSpec, SpaceArrangement,
    WindowPlacement,
};
use ds_core::vocab::Fraction;

fn min_width(w: u32) -> SizeLimits {
    SizeLimits {
        min: size(w, 0),
        max: MaxSize::Unbounded,
    }
}

/// A Split View of windows 1 (left) and 2 over `full`, with the usable area smaller (a bar).
fn split(full: LogicalRect, share: u16, lead: SizeLimits, trail: SizeLimits) -> SceneSpec {
    let usable = rect(
        full.origin.x,
        full.origin.y + 24,
        full.size.w,
        full.size.h - 24,
    );
    let windows = vec![
        window(1, WindowPlacement::FullScreen, lead),
        window(2, WindowPlacement::FullScreen, trail),
    ];
    let mut spec = one_desktop(usable, windows);
    spec.outputs[0].full = full;
    spec.outputs[0].spaces[0].kind = SpaceArrangement::Split(SplitPair {
        leading: WindowId(1),
        trailing: WindowId(2),
        divider: Fraction(share),
    });
    spec
}

fn rects(arrangement: &Arrangement) -> Vec<(u64, LogicalRect)> {
    arrangement
        .windows
        .iter()
        .map(|target| (target.window.0, target.rect))
        .collect()
}

#[test]
fn the_halves_share_the_output_by_the_dividers_share_and_the_minimums() {
    let full = rect(0, 0, 1920, 1080);
    // 1912 px are left for the halves once the 8 px divider is taken out.
    let cases = [
        ("the middle", 500, ANY_SIZE, ANY_SIZE, (956, 956)),
        (
            "a narrow leading half",
            300,
            ANY_SIZE,
            ANY_SIZE,
            (573, 1339),
        ),
        (
            "the leading minimum wins",
            300,
            min_width(1000),
            ANY_SIZE,
            (1000, 912),
        ),
        (
            "the trailing minimum wins",
            500,
            ANY_SIZE,
            min_width(1500),
            (412, 1500),
        ),
        (
            "minimums that cannot both fit",
            500,
            min_width(1500),
            min_width(1500),
            (956, 956),
        ),
    ];
    for (name, share, lead, trail, (lead_w, trail_w)) in cases {
        let got = RustScene.arrange(&split(full, share, lead, trail), &knobs());
        let want = vec![
            (1, rect(0, 0, lead_w, 1080)),
            (
                2,
                rect(i32::try_from(lead_w + 8).expect("small"), 0, trail_w, 1080),
            ),
        ];
        assert_eq!(rects(&got), want, "{name}");
        assert!(
            got.windows
                .iter()
                .all(|t| t.size == ConfigureNeed::Resize(t.rect.size)),
            "{name}: each half is asked for its size"
        );
    }
}

#[test]
fn a_minimum_that_cannot_be_met_is_reported() {
    let full = rect(0, 0, 1920, 1080);
    let got = RustScene.arrange(
        &split(full, 500, min_width(1500), min_width(1500)),
        &knobs(),
    );
    let unmet: Vec<WindowId> = got
        .reports
        .iter()
        .filter_map(|report| match report {
            LayoutReport::MinUnmet { window, .. } => Some(*window),
            _ => None,
        })
        .collect();
    assert_eq!(unmet, vec![WindowId(1), WindowId(2)]);
    let fine = RustScene.arrange(&split(full, 500, ANY_SIZE, ANY_SIZE), &knobs());
    assert!(fine.reports.is_empty());
}

#[test]
fn the_halves_follow_the_outputs_place() {
    let full = rect(1920, 100, 1000, 500);
    let got = RustScene.arrange(&split(full, 500, ANY_SIZE, ANY_SIZE), &knobs());
    assert_eq!(
        rects(&got),
        vec![
            (1, rect(1920, 100, 496, 500)),
            (2, rect(2424, 100, 496, 500))
        ]
    );
}

#[test]
fn a_pair_with_a_window_the_space_lacks_gets_no_target() {
    let mut spec = split(rect(0, 0, 1920, 1080), 500, ANY_SIZE, ANY_SIZE);
    spec.outputs[0].spaces[0].windows.truncate(1);
    let got = RustScene.arrange(&spec, &knobs());
    assert!(got.windows.is_empty());
}
