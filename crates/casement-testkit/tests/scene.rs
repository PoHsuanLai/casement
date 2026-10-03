//! The ten base scenes (P0-R-scene, tier P: pixman, byte-exact) and the blur v0 scene
//! (tier L: llvmpipe), driven through the fixture so real client surfaces are drawn.
//! Goldens live in `tests/golden/scenes/<name>.png`, blessed with `CASEMENT_BLESS=1`.

use casement_backend::SoftRenderer;
use casement_render::golden::{Golden, GoldenOutcome, GoldenTier, compare};
use casement_testkit::{Env, Fixture, FixtureSpec};

/// The P0 base scenes: name, outputs.
const SCENES: [(&str, &[&str]); 10] = [
    ("empty-desktop-1x", &["HEADLESS-1:1920x1080@60000:120:0,0"]),
    ("one-window-1x", &["HEADLESS-1:1920x1080@60000:120:0,0"]),
    ("one-window-1.5x", &["HEADLESS-1:2880x1620@60000:180:0,0"]),
    ("one-window-2x", &["HEADLESS-1:3840x2160@60000:240:0,0"]),
    (
        "cascaded-windows-1x",
        &["HEADLESS-1:1920x1080@60000:120:0,0"],
    ),
    (
        "popup-over-window-1x",
        &["HEADLESS-1:1920x1080@60000:120:0,0"],
    ),
    ("bar-and-window-1x", &["HEADLESS-1:1920x1080@60000:120:0,0"]),
    ("lock-surface-1x", &["HEADLESS-1:1920x1080@60000:120:0,0"]),
    (
        "lock-cover-dead-locker-1x",
        &["HEADLESS-1:1920x1080@60000:120:0,0"],
    ),
    ("named-cursor-1.5x", &["HEADLESS-1:2880x1620@60000:180:0,0"]),
];

/// Builds scene `name` on `fixture` with test clients and shoots its output.
fn shoot(fixture: &mut Fixture, name: &str) -> image::RgbaImage {
    let _ = fixture;
    todo!("P0-R-scene: build scene {name} with test clients and shoot it")
}

#[test]
#[ignore = "P0-R-scene"]
fn the_ten_base_scenes_match_their_pixman_goldens() {
    let env = Env::of_process();
    for (name, outputs) in SCENES {
        let mut fixture = Fixture::start(FixtureSpec::desktop().with_outputs(outputs)).expect(name);
        let actual = shoot(&mut fixture, name);
        let golden = Golden {
            scene: format!("tests/golden/scenes/{name}"),
            tier: GoldenTier::Exact,
        };
        let outcome = compare(&golden, &actual, &env.out_dir, env.bless);
        assert!(
            matches!(outcome, GoldenOutcome::Match | GoldenOutcome::Blessed),
            "{name}: {outcome:?}"
        );
    }
}

#[test]
#[ignore = "P0-R-scene"]
fn blur_v0_matches_its_llvmpipe_golden() {
    let env = Env::of_process();
    let spec = FixtureSpec::desktop().with_renderer(SoftRenderer::Llvmpipe);
    let mut fixture = Fixture::start(spec).expect("fixture");
    let actual = shoot(&mut fixture, "blur-v0-1x");
    let golden = Golden {
        scene: "tests/golden/scenes/blur-v0-1x".to_owned(),
        tier: GoldenTier::Soft {
            channel: 2,
            parts_per_million: 1000,
        },
    };
    let outcome = compare(&golden, &actual, &env.out_dir, env.bless);
    assert!(
        matches!(outcome, GoldenOutcome::Match | GoldenOutcome::Blessed),
        "{outcome:?}"
    );
}
