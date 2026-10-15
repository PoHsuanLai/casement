//! S-L6: CssScene and RustScene agree on 500 generated scenes.
//!
//! The specs come from a fixed seed (splitmix64, no proptest in this crate's dependencies), so a
//! failure names a spec index that reproduces everywhere. The comparison is exact: both
//! implementations lay out in whole logical pixels, and `read.rs` rounds the f32 boxes to the
//! nearest pixel, which is the only tolerance.
//!
//! What CSS cannot say is excluded by name, never by loosening the comparison:
//!
//! - [`SPREAD`]: Mission Control's slots and `SpreadOverflow` are dropped from both sides; the
//!   slots are checked only for covering exactly the windows that have a desktop target.
//! - [`TILED`]: no spec holds a `Tiled` window (it needs `casement-protocols`, which this crate
//!   does not depend on); `RustScene` gives it no target at all.
//! - [`CASCADE_STEP`]: every spec's cascade step is one positive length on both axes.
//! - [`APP_EXPOSE`] and [`SHOW_DESKTOP`]: `RustScene` arranges them as plain desktops today
//!   (they arrive with P3-Z), so they are generated and compared as such.
//!
//! The same list is in FINDINGS.md, "casement-scene-css: the exclusions".

use casement_core::app::AppKey;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::output::{EdidFacts, EdidSerial};
use casement_core::settings::CasementSettings;
use casement_core::space::SplitPair;
use casement_core::window::{MaxSize, SizeLimits};
use casement_core::{OutputKey, OutputName, WindowId};
use casement_layout::{
    Arrangement, LayoutKnobs, LayoutReport, OutputSpec, OverviewKind, RustScene, SceneLayout,
    SceneMode, SceneSpec, SpaceArrangement, SpaceSpec, StackIndex, WindowPlacement, WindowSpec,
};
use casement_scene_css::CssScene;
use ds_core::geometry::scale::Scale;
use porter_core::{ClaimedId, SpaceId};

/// Mission Control's slots and its `SpreadOverflow` report: an iterative push-apart with a
/// shelf-pack fallback, which no CSS expression states. Dropped from both arrangements.
const SPREAD: &str = "mission-control spread";

/// `WindowPlacement::Tiled` is not generated (a tile zone is a `casement-protocols` type).
const TILED: &str = "tiled windows";

/// Only equal positive cascade steps are generated (`scene.css` walks one length).
const CASCADE_STEP: &str = "cascade step";

/// App Exposé is arranged as a desktop by `RustScene` today.
const APP_EXPOSE: &str = "app expose";

/// Show Desktop is arranged as a desktop by `RustScene` today.
const SHOW_DESKTOP: &str = "show desktop";

/// The exclusions, by name.
const EXCLUSIONS: [&str; 5] = [SPREAD, TILED, CASCADE_STEP, APP_EXPOSE, SHOW_DESKTOP];

/// How many specs.
const SPECS: u64 = 500;

/// The seed every spec's own generator derives from.
const SEED: u64 = 0x5CE1_0C55_0000_0006;

/// splitmix64: a small deterministic generator.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..n` (n > 0).
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// A value in `lo..=hi`.
    fn between(&mut self, lo: u32, hi: u32) -> u32 {
        lo + u32::try_from(self.below(u64::from(hi - lo) + 1)).unwrap_or(0)
    }

    /// A signed value in `lo..=hi`.
    fn signed(&mut self, lo: i32, hi: i32) -> i32 {
        let span = u32::try_from(i64::from(hi) - i64::from(lo)).unwrap_or(0);
        lo + i32::try_from(self.between(0, span)).unwrap_or(0)
    }

    /// Whether a one-in-`n` chance came up.
    fn one_in(&mut self, n: u64) -> bool {
        self.below(n) == 0
    }
}

fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

fn output_key(name: &str) -> OutputKey {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    OutputKey::of(&edid, &OutputName::parse(name).expect("name"))
}

fn space_id(n: u64) -> SpaceId {
    SpaceId::parse(&format!("s-{n}")).expect("space id")
}

fn app(rng: &mut Rng) -> AppKey {
    let name = ["foot", "mailo", "anyview"][usize::try_from(rng.below(3)).unwrap_or(0)];
    AppKey::Claimed(ClaimedId::parse(name).expect("claimed id"))
}

fn limits(rng: &mut Rng) -> SizeLimits {
    let min = match rng.one_in(3) {
        true => LogicalSize { w: 0, h: 0 },
        false => LogicalSize {
            w: rng.between(0, 900),
            h: rng.between(0, 700),
        },
    };
    let max = match rng.below(3) {
        0 => MaxSize::Unbounded,
        1 => MaxSize::Bounded(LogicalSize {
            w: rng.between(0, 2600),
            h: rng.between(0, 1600),
        }),
        _ => MaxSize::Bounded(LogicalSize {
            w: rng.between(300, 1200),
            h: rng.between(200, 900),
        }),
    };
    SizeLimits { min, max }
}

fn size(rng: &mut Rng) -> LogicalSize {
    match rng.one_in(8) {
        true => LogicalSize {
            w: rng.between(0, 5000),
            h: rng.between(0, 4000),
        },
        false => LogicalSize {
            w: rng.between(1, 1600),
            h: rng.between(1, 1100),
        },
    }
}

fn placement(rng: &mut Rng) -> WindowPlacement {
    match rng.below(7) {
        0 | 1 => WindowPlacement::Floating(rect_of(rng)),
        2 => WindowPlacement::Placed(rect_of(rng)),
        3 => WindowPlacement::Filled,
        4 => WindowPlacement::FullScreen,
        5 => WindowPlacement::Minimised,
        _ => WindowPlacement::New(size(rng)),
    }
}

fn rect_of(rng: &mut Rng) -> LogicalRect {
    let size = size(rng);
    LogicalRect::new(
        LogicalPoint {
            x: rng.signed(-600, 3200),
            y: rng.signed(-600, 2000),
        },
        size,
    )
}

fn window(rng: &mut Rng, next_window: &mut u64, stack: u32) -> WindowSpec {
    *next_window += 1;
    let reserve_top = match rng.below(6) {
        0 | 1 => 28,
        2 if rng.one_in(8) => rng.between(0, 3000),
        _ => 0,
    };
    WindowSpec {
        window: WindowId(*next_window),
        app: app(rng),
        placement: placement(rng),
        limits: limits(rng),
        stack: StackIndex(stack),
        reserve_top,
    }
}

/// A divider's share, from `knobs`' own `Fraction` (this crate does not name `ds-vocab`'s).
fn pair(rng: &mut Rng, ids: [WindowId; 2], knobs: &LayoutKnobs) -> SplitPair {
    let mut divider = knobs.spread_floor;
    divider.0 = u16::try_from(rng.between(0, 1100)).unwrap_or(500);
    SplitPair {
        leading: ids[0],
        trailing: ids[1],
        divider,
    }
}

fn space(
    rng: &mut Rng,
    next_space: &mut u64,
    next_window: &mut u64,
    knobs: &LayoutKnobs,
) -> SpaceSpec {
    *next_space += 1;
    let count = rng.between(0, 5);
    let windows: Vec<WindowSpec> = (0..count)
        .map(|stack| window(rng, next_window, stack))
        .collect();
    let ids: Vec<WindowId> = windows.iter().map(|w| w.window).collect();
    let kind = match (rng.below(10), ids.as_slice()) {
        (0..=1, [first, second, ..]) => {
            // One time in five the pair names a window the Space does not hold.
            let second = if rng.one_in(5) {
                WindowId(u64::MAX)
            } else {
                *second
            };
            SpaceArrangement::Split(pair(rng, [*first, second], knobs))
        }
        (2, [only, ..]) => SpaceArrangement::FullScreen(*only),
        _ => SpaceArrangement::Desktop,
    };
    SpaceSpec {
        space: space_id(*next_space),
        kind,
        windows,
    }
}

fn mode(rng: &mut Rng, spaces: &[SpaceSpec]) -> SceneMode {
    let shown = match rng.one_in(8) {
        true => space_id(9999),
        false => spaces[usize::try_from(rng.below(spaces.len() as u64)).unwrap_or(0)]
            .space
            .clone(),
    };
    match rng.below(6) {
        0 => SceneMode::ShowDesktop,
        1 => SceneMode::Overview {
            shown,
            kind: OverviewKind::AppExpose(app(rng)),
        },
        2 | 3 => SceneMode::Overview {
            shown,
            kind: OverviewKind::MissionControl,
        },
        _ => SceneMode::Desktop { shown },
    }
}

fn output(
    rng: &mut Rng,
    index: u64,
    next_space: &mut u64,
    next_window: &mut u64,
    knobs: &LayoutKnobs,
) -> OutputSpec {
    let full = rect(
        i32::try_from(index).unwrap_or(0) * 3000,
        0,
        rng.between(300, 2800),
        rng.between(200, 1600),
    );
    let (left, top) = (rng.between(0, 80), rng.between(0, 60));
    let usable = rect(
        full.origin.x + i32::try_from(left).unwrap_or(0),
        full.origin.y + i32::try_from(top).unwrap_or(0),
        full.size.w.saturating_sub(left + rng.between(0, 80)),
        full.size.h.saturating_sub(top + rng.between(0, 60)),
    );
    let spaces: Vec<SpaceSpec> = (0..rng.between(1, 3))
        .map(|_| space(rng, next_space, next_window, knobs))
        .collect();
    OutputSpec {
        output: output_key(&format!("HEADLESS-{}", index + 1)),
        full,
        usable,
        scale: Scale::ONE,
        mode: mode(rng, &spaces),
        spaces,
    }
}

/// The knobs for spec `n`: the production ones, with one positive cascade step on both axes
/// (`CASCADE_STEP`).
fn knobs(rng: &mut Rng) -> LayoutKnobs {
    let mut knobs = LayoutKnobs::of(&CasementSettings::defaults());
    let step = i32::try_from(rng.between(1, 80)).unwrap_or(28);
    knobs.cascade_step = LogicalPoint { x: step, y: step };
    knobs
}

/// Spec `n` and its knobs.
fn scene(n: u64) -> (SceneSpec, LayoutKnobs) {
    let mut rng = Rng(SEED ^ n.wrapping_mul(0xD1B5_4A32_D192_ED03));
    let knobs = knobs(&mut rng);
    let (mut next_space, mut next_window) = (0, 0);
    let outputs = (0..rng.between(1, 2))
        .map(|index| {
            output(
                &mut rng,
                u64::from(index),
                &mut next_space,
                &mut next_window,
                &knobs,
            )
        })
        .collect();
    (SceneSpec { outputs }, knobs)
}

/// `arrangement` without what `SPREAD` excludes.
fn without_spread(mut arrangement: Arrangement) -> Arrangement {
    arrangement.overview.clear();
    arrangement
        .reports
        .retain(|report| !matches!(report, LayoutReport::SpreadOverflow(_)));
    arrangement
}

/// `SPREAD`'s one check: the slots are the windows that have a desktop target, one each, on
/// the outputs in Mission Control, and nowhere else.
fn spread_slots_cover_the_shown_windows(spec: &SceneSpec, rust: &Arrangement) -> bool {
    let mut want: Vec<WindowId> = Vec::new();
    for output in &spec.outputs {
        let SceneMode::Overview {
            shown,
            kind: OverviewKind::MissionControl,
        } = &output.mode
        else {
            continue;
        };
        let Some(space) = output.spaces.iter().find(|space| &space.space == shown) else {
            continue;
        };
        if matches!(space.kind, SpaceArrangement::Desktop) {
            want.extend(
                space
                    .windows
                    .iter()
                    .map(|w| w.window)
                    .filter(|id| rust.windows.iter().any(|t| t.window == *id)),
            );
        }
    }
    let mut got: Vec<WindowId> = rust.overview.iter().map(|slot| slot.window).collect();
    want.sort();
    got.sort();
    want == got
}

#[test]
fn css_scene_matches_rust_scene_over_500_specs() {
    for n in 0..SPECS {
        let (spec, knobs) = scene(n);
        let rust = RustScene.arrange(&spec, &knobs);
        let css = CssScene.arrange(&spec, &knobs);
        assert!(
            spread_slots_cover_the_shown_windows(&spec, &rust),
            "spec {n} ({SPREAD}): the slots are not the shown windows\n{spec:#?}\n{rust:#?}"
        );
        assert_eq!(
            css,
            without_spread(rust),
            "spec {n}: CssScene differs from RustScene\n{spec:#?}\n{knobs:?}"
        );
    }
}

/// The generator must reach every kind `CssScene` is claimed to lay out, so that an all-green
/// run cannot be an empty one.
#[test]
fn the_specs_reach_every_laid_out_kind() {
    let mut seen = [0_u32; 9];
    for n in 0..SPECS {
        let (spec, knobs) = scene(n);
        let arranged = RustScene.arrange(&spec, &knobs);
        seen[0] += u32::try_from(arranged.dividers.len()).unwrap_or(0);
        seen[1] += u32::try_from(arranged.backdrops.len()).unwrap_or(0);
        seen[2] += u32::try_from(arranged.reports.len()).unwrap_or(0);
        seen[3] += u32::try_from(arranged.overview.len()).unwrap_or(0);
        for output in &spec.outputs {
            for space in &output.spaces {
                seen[4] += u32::from(matches!(space.kind, SpaceArrangement::FullScreen(_)));
                for window in &space.windows {
                    match window.placement {
                        WindowPlacement::Floating(_) => seen[5] += 1,
                        WindowPlacement::New(_) => seen[6] += 1,
                        WindowPlacement::Filled => seen[7] += 1,
                        WindowPlacement::FullScreen => seen[8] += 1,
                        WindowPlacement::Placed(_)
                        | WindowPlacement::Minimised
                        | WindowPlacement::Tiled(_) => {}
                    }
                }
            }
        }
    }
    let names = [
        "dividers",
        "backdrops",
        "reports",
        "spread slots",
        "full-screen spaces",
        "floating",
        "new",
        "filled",
        "full-screen windows",
    ];
    for (name, count) in names.iter().zip(seen) {
        assert!(count > 0, "no generated spec has {name}");
    }
    assert_eq!(
        EXCLUSIONS.len(),
        6,
        "the exclusions are listed in FINDINGS.md"
    );
}
