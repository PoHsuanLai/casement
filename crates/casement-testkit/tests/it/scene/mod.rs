//! The base scenes (and the vkms one) (P0-R-scene, tier P: pixman, byte-exact) and the blur v0 scene
//! (tier L: llvmpipe), driven through the fixture so real client surfaces are drawn.
//! Goldens live in `tests/golden/scenes/<name>.png`, blessed with `CASEMENT_BLESS=1`.

use casement_backend::SoftRenderer;
use casement_render::golden::{Golden, GoldenOutcome, GoldenTier, compare};
use casement_testkit::{Env, Fixture, FixtureSpec};

/// The P0 base scenes: name, outputs.
const SCENES: [(&str, &[&str]); 11] = [
    ("empty-desktop-1x", &["HEADLESS-1:1920x1080@60000:120:0,0"]),
    // The vkms writeback golden (dev/vkms/accept.sh): the default 1024x768 mode of a vkms CRTC.
    (
        "empty-desktop-1024x768",
        &["HEADLESS-1:1024x768@60000:120:0,0"],
    ),
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

mod build;
mod client_round;
mod clip;
mod desk;
mod frames;
mod minimise;

/// Compares `actual` with the pixman golden `tests/golden/scenes/<name>.png`; with
/// `CASEMENT_BLESS=1` the golden is written (deliberate, with a reason in the commit).
fn golden_matches(name: &str, actual: &image::RgbaImage) {
    let env = Env::of_process();
    let golden = Golden {
        scene: format!("tests/golden/scenes/{name}"),
        tier: GoldenTier::Exact,
    };
    let outcome = compare(&golden, actual, &env.out_dir, env.bless);
    assert!(
        matches!(outcome, GoldenOutcome::Match | GoldenOutcome::Blessed),
        "{name}: {outcome:?}"
    );
}

/// Builds scene `name` on `fixture` with test clients and shoots its output.
fn shoot(fixture: &mut Fixture, name: &str) -> image::RgbaImage {
    build::scene(fixture, name)
}

#[test]
fn the_base_scenes_match_their_pixman_goldens() {
    let env = Env::of_process();
    let failures: Vec<String> = SCENES
        .iter()
        .filter_map(|(name, outputs)| {
            let mut fixture =
                Fixture::start(FixtureSpec::desktop().with_outputs(outputs)).expect(name);
            let actual = shoot(&mut fixture, name);
            let golden = Golden {
                scene: format!("tests/golden/scenes/{name}"),
                tier: GoldenTier::Exact,
            };
            match compare(&golden, &actual, &env.out_dir, env.bless) {
                GoldenOutcome::Match | GoldenOutcome::Blessed => None,
                outcome => Some(format!("{name}: {outcome:?}")),
            }
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
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
