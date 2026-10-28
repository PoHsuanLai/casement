//! The Split View divider as the pointer sees it: the hit on the strip and the share a pointer
//! column means.

use crate::support::scene::{ANY_SIZE, knobs, rect, size, window};
use casement_core::WindowId;
use casement_core::geometry::LogicalPoint;
use casement_core::space::SplitPair;
use casement_core::window::SizeLimits;
use casement_layout::{
    DividerTarget, RustScene, SceneLayout, SceneSpec, SpaceArrangement, WindowPlacement,
};
use ds_core::vocab::Fraction;

fn min_width(w: u32) -> SizeLimits {
    SizeLimits {
        min: size(w, 0),
        max: None,
    }
}

/// The divider layout makes for a 1920 wide output at the middle share.
fn divider(lead: SizeLimits, trail: SizeLimits) -> DividerTarget {
    let full = rect(0, 0, 1920, 1080);
    let windows = vec![
        window(1, WindowPlacement::FullScreen, lead),
        window(2, WindowPlacement::FullScreen, trail),
    ];
    let mut spec: SceneSpec = crate::support::scene::one_desktop(full, windows);
    spec.outputs[0].spaces[0].kind = SpaceArrangement::Split(SplitPair {
        leading: WindowId(1),
        trailing: WindowId(2),
        divider: Fraction(500),
    });
    let got = RustScene.arrange(&spec, &knobs());
    got.dividers.into_iter().next().expect("a divider")
}

#[test]
fn the_strip_takes_the_pointer_and_nothing_beside_it() {
    let target = divider(ANY_SIZE, ANY_SIZE);
    // name, point, hit
    let cases = [
        ("the strip's first column", (956, 500), true),
        ("the strip's last column", (963, 500), true),
        ("just left of it", (955, 500), false),
        ("just right of it", (964, 500), false),
        ("the top row", (960, 0), true),
        ("below the output", (960, 1080), false),
    ];
    for (name, (x, y), want) in cases {
        assert_eq!(target.hit(LogicalPoint { x, y }), want, "{name}");
    }
}

#[test]
fn the_share_follows_the_pointer_within_its_limits() {
    // The row is 1920 wide with an 8 px strip: 1912 px for the halves; the strip's centre is
    // 4 px into the strip.
    let cases = [
        ("the middle", ANY_SIZE, ANY_SIZE, 960, 500),
        ("a third", ANY_SIZE, ANY_SIZE, 641, 333),
        ("not past the near limit", ANY_SIZE, ANY_SIZE, 10, 200),
        ("not past the far limit", ANY_SIZE, ANY_SIZE, 1910, 800),
        (
            "a pointer left of the output",
            ANY_SIZE,
            ANY_SIZE,
            -500,
            200,
        ),
        // 1000 px of 1912 is 523.01 thousandths: 524 is the least that leaves it.
        ("the leading minimum", min_width(1000), ANY_SIZE, 600, 524),
        // 1500 px for the trailing half leaves 412 of 1912: 215 thousandths at most.
        ("the trailing minimum", ANY_SIZE, min_width(1500), 1500, 215),
        // The minimums cannot both fit inside 200..800: only the limits hold.
        (
            "minimums that cannot fit",
            min_width(1500),
            min_width(1500),
            1000,
            521,
        ),
        (
            "minimums wider than the limits",
            min_width(1900),
            ANY_SIZE,
            100,
            200,
        ),
    ];
    for (name, lead, trail, x, want) in cases {
        let target = divider(lead, trail);
        assert_eq!(target.share_at(x), Fraction(want), "{name}");
    }
}
