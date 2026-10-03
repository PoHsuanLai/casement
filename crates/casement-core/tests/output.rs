//! Output policy (what P0-F1-output makes pass): the first-sight scale, mode choice and
//! placement. The scale rows are sill's PreferWhole table (`sill-model` `displays/scale.rs`),
//! in 120ths.

mod support;

use casement_core::geometry::{LogicalPoint, PhysicalSize, Transform};
use casement_core::output::{
    ModePreference, ModeWish, OutputConfig, OutputPower, SavedOutputs, ScalePolicy,
    first_sight_scale, pick_mode, place_outputs,
};
use ds_core::geometry::scale::Scale;
use std::collections::BTreeMap;
use support::{display, headless, mode};

const PREFERRED: ModePreference = ModePreference::Preferred;
const OTHER: ModePreference = ModePreference::Other;

#[test]
fn first_sight_scales() {
    let one = |w, h, mm| display("DP-1", vec![mode(w, h, 60_000, PREFERRED)], mm);
    let cases = [
        (
            "27-inch 4K: 2x, looks like 1920x1080",
            one(3840, 2160, (597, 336)),
            ScalePolicy::WholeLikeMac,
            Scale(240),
        ),
        (
            "27-inch 1440p: 1x",
            one(2560, 1440, (597, 336)),
            ScalePolicy::WholeLikeMac,
            Scale(120),
        ),
        (
            "24-inch 1080p: 1x",
            one(1920, 1080, (531, 299)),
            ScalePolicy::WholeLikeMac,
            Scale(120),
        ),
        (
            "13-inch 2560x1600: 2x",
            one(2560, 1600, (286, 179)),
            ScalePolicy::WholeLikeMac,
            Scale(240),
        ),
        (
            "14-inch 1920x1200: ideal under 1.5, nearest step 1.5x",
            one(1920, 1200, (302, 189)),
            ScalePolicy::WholeLikeMac,
            Scale(180),
        ),
        (
            "4K with no size: 2x",
            one(3840, 2160, (0, 0)),
            ScalePolicy::WholeLikeMac,
            Scale(240),
        ),
        (
            "headless 1080p: 1x",
            headless("HEADLESS-1"),
            ScalePolicy::WholeLikeMac,
            Scale(120),
        ),
        (
            "a fixed policy wins",
            one(3840, 2160, (597, 336)),
            ScalePolicy::Fixed(Scale(150)),
            Scale(150),
        ),
    ];
    for (name, facts, policy, want) in cases {
        assert_eq!(first_sight_scale(&facts, &policy), want, "{name}");
    }
}

#[test]
fn picking_modes() {
    let facts = display(
        "DP-1",
        vec![
            mode(3840, 2160, 60_000, PREFERRED),
            mode(3840, 2160, 144_000, OTHER),
            mode(2560, 1440, 60_000, OTHER),
            mode(1920, 1080, 240_000, OTHER),
        ],
        (597, 336),
    );
    let cases = [
        (
            "preferred",
            ModeWish::Preferred,
            mode(3840, 2160, 60_000, PREFERRED),
        ),
        (
            "an offered mode",
            ModeWish::Exact(mode(2560, 1440, 60_000, OTHER)),
            mode(2560, 1440, 60_000, OTHER),
        ),
        (
            "a mode not offered falls back",
            ModeWish::Exact(mode(1280, 720, 60_000, OTHER)),
            mode(3840, 2160, 60_000, PREFERRED),
        ),
        (
            "fastest at a size",
            ModeWish::HighestRefreshAt(PhysicalSize { w: 3840, h: 2160 }),
            mode(3840, 2160, 144_000, OTHER),
        ),
        (
            "a size not offered falls back",
            ModeWish::HighestRefreshAt(PhysicalSize { w: 800, h: 600 }),
            mode(3840, 2160, 60_000, PREFERRED),
        ),
    ];
    for (name, wish, want) in cases {
        assert_eq!(pick_mode(&facts, &wish), want, "{name}");
    }
    let unmarked = display(
        "DP-2",
        vec![
            mode(2560, 1440, 60_000, OTHER),
            mode(3840, 2160, 30_000, OTHER),
            mode(3840, 2160, 60_000, OTHER),
        ],
        (0, 0),
    );
    assert_eq!(
        pick_mode(&unmarked, &ModeWish::Preferred),
        mode(3840, 2160, 60_000, OTHER),
        "largest, then fastest"
    );
}

#[test]
fn placing_outputs() {
    let small = headless("HEADLESS-1");
    let big = display(
        "DP-1",
        vec![mode(3840, 2160, 60_000, PREFERRED)],
        (597, 336),
    );
    let fresh = |m, scale, x| OutputConfig {
        mode: m,
        scale: Scale(scale),
        transform: Transform::Normal,
        position: LogicalPoint { x, y: 0 },
        power: OutputPower::On,
    };
    let got = place_outputs(
        &[small.clone(), big.clone()],
        &SavedOutputs::default(),
        &ScalePolicy::WholeLikeMac,
    );
    assert_eq!(
        got,
        vec![
            (
                small.key.clone(),
                fresh(mode(1920, 1080, 60_000, PREFERRED), 120, 0)
            ),
            (
                big.key.clone(),
                fresh(mode(3840, 2160, 60_000, PREFERRED), 240, 1920)
            ),
        ],
        "new outputs in a row"
    );
    let remembered = OutputConfig {
        scale: Scale(180),
        position: LogicalPoint { x: -1920, y: 0 },
        ..fresh(mode(3840, 2160, 60_000, PREFERRED), 180, 0)
    };
    let saved = SavedOutputs(BTreeMap::from([(big.key.clone(), remembered)]));
    let got = place_outputs(
        &[small.clone(), big.clone()],
        &saved,
        &ScalePolicy::WholeLikeMac,
    );
    assert_eq!(
        got,
        vec![
            (
                small.key,
                fresh(mode(1920, 1080, 60_000, PREFERRED), 120, 640)
            ),
            (big.key, remembered)
        ],
        "a saved display keeps its place; a new one goes right of it"
    );
}
