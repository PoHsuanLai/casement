//! Golden images (feature `testing`): compare a rendered frame with its blessed PNG.
//!
//! Tiers (11 §2.8 as SP2 corrected it, A18): `Exact` is byte-identical (pixman against
//! pixman, and aligned elements across pixman and llvmpipe); `Soft` allows `channel` steps per
//! channel on at most `parts_per_million` of the pixels (the L tier: 2 per channel, at most
//! 1000 ppm, GLES on llvmpipe against itself). Blessing is deliberate: the test kit's
//! environment reads `CASEMENT_BLESS`, never this module.

use image::{Rgba, RgbaImage};
use std::path::{Path, PathBuf};

/// One golden: where its PNG lives and how close a frame must be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Golden {
    /// The golden's path without `.png`, relative to the test's working directory (the crate
    /// root under nextest): `tests/golden/scenes/two-windows-1x`.
    pub scene: String,
    /// How close a frame must be.
    pub tier: GoldenTier,
}

/// How close a frame must be to its golden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GoldenTier {
    /// Every byte equal.
    Exact,
    /// At most `parts_per_million` of the pixels differ by more than `channel` in any channel.
    Soft {
        /// The largest per-channel difference that still counts as equal.
        channel: u8,
        /// How many pixels in a million may differ by more.
        parts_per_million: u32,
    },
}

/// What a comparison found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoldenOutcome {
    /// The frame matches its golden.
    Match,
    /// The frame was written as the new golden.
    Blessed,
    /// The frame differs (or the golden is missing): the frame and a diff image were written.
    Differs {
        /// The frame as rendered.
        actual: PathBuf,
        /// Differing pixels in red over a dimmed copy of the frame (the frame itself when the
        /// sizes differ or the golden is missing).
        diff: PathBuf,
    },
}

/// Compares `actual` with `golden`'s PNG. A difference writes `<name>.actual.png` and
/// `<name>.diff.png` into `out_dir` (`<name>` is the scene's last path component), or with
/// `bless` replaces the golden (`CASEMENT_BLESS=1`, with a reason in the commit). A write that fails still reports
/// [`GoldenOutcome::Differs`], naming the paths it tried.
pub fn compare(golden: &Golden, actual: &RgbaImage, out_dir: &Path, bless: bool) -> GoldenOutcome {
    let golden_path = PathBuf::from(format!("{}.png", golden.scene));
    let expected = image::open(&golden_path).ok().map(|image| image.to_rgba8());
    match (verdict(expected.as_ref(), actual, golden.tier), bless) {
        (Comparison::Same, _) => GoldenOutcome::Match,
        (Comparison::Differs(_), true) => bless_into(&golden_path, actual),
        (Comparison::Differs(diff), false) => report(golden, actual, &diff, out_dir),
    }
}

/// Whether two frames are equal under a tier, and if not, the diff image.
enum Comparison {
    Same,
    Differs(RgbaImage),
}

fn verdict(expected: Option<&RgbaImage>, actual: &RgbaImage, tier: GoldenTier) -> Comparison {
    match expected {
        Some(expected) if expected.dimensions() == actual.dimensions() => {
            let differing = differing_pixels(expected, actual, tier);
            if within(tier, differing.len(), pixel_count(actual)) {
                Comparison::Same
            } else {
                Comparison::Differs(diff_image(actual, &differing))
            }
        }
        _ => Comparison::Differs(actual.clone()),
    }
}

/// The pixels that differ by more than the tier's per-channel allowance.
fn differing_pixels(expected: &RgbaImage, actual: &RgbaImage, tier: GoldenTier) -> Vec<(u32, u32)> {
    let allowance = match tier {
        GoldenTier::Exact => 0,
        GoldenTier::Soft { channel, .. } => channel,
    };
    expected
        .enumerate_pixels()
        .zip(actual.pixels())
        .filter(|((_, _, e), a)| {
            e.0.iter()
                .zip(a.0.iter())
                .any(|(e, a)| e.abs_diff(*a) > allowance)
        })
        .map(|((x, y, _), _)| (x, y))
        .collect()
}

fn pixel_count(image: &RgbaImage) -> u64 {
    u64::from(image.width()) * u64::from(image.height())
}

/// Whether `differing` of `total` pixels is within the tier.
fn within(tier: GoldenTier, differing: usize, total: u64) -> bool {
    let differing = differing as u64;
    match tier {
        GoldenTier::Exact => differing == 0,
        GoldenTier::Soft {
            parts_per_million, ..
        } => {
            differing.saturating_mul(1_000_000)
                <= u64::from(parts_per_million).saturating_mul(total)
        }
    }
}

/// `actual` dimmed to a quarter, with every differing pixel in opaque red.
fn diff_image(actual: &RgbaImage, differing: &[(u32, u32)]) -> RgbaImage {
    let dimmed = RgbaImage::from_fn(actual.width(), actual.height(), |x, y| {
        let Rgba([r, g, b, a]) = *actual.get_pixel(x, y);
        Rgba([r / 4, g / 4, b / 4, a])
    });
    differing.iter().fold(dimmed, |mut image, &(x, y)| {
        image.put_pixel(x, y, Rgba([255, 0, 0, 255]));
        image
    })
}

fn bless_into(golden_path: &Path, actual: &RgbaImage) -> GoldenOutcome {
    let written = golden_path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .ok()
        .and_then(|()| actual.save(golden_path).ok());
    match written {
        Some(()) => GoldenOutcome::Blessed,
        None => GoldenOutcome::Differs {
            actual: golden_path.to_path_buf(),
            diff: golden_path.to_path_buf(),
        },
    }
}

fn report(golden: &Golden, actual: &RgbaImage, diff: &RgbaImage, out_dir: &Path) -> GoldenOutcome {
    let name = Path::new(&golden.scene)
        .file_name()
        .map_or_else(|| "golden".to_owned(), |n| n.to_string_lossy().into_owned());
    let actual_path = out_dir.join(format!("{name}.actual.png"));
    let diff_path = out_dir.join(format!("{name}.diff.png"));
    // A failed write still reports the difference: the paths say where it tried.
    let _ = std::fs::create_dir_all(out_dir)
        .ok()
        .and_then(|()| actual.save(&actual_path).ok())
        .and_then(|()| diff.save(&diff_path).ok());
    GoldenOutcome::Differs {
        actual: actual_path,
        diff: diff_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(w: u32, h: u32, value: u8) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba([value, value, value, 255]))
    }

    fn golden_in(dir: &Path, tier: GoldenTier) -> Golden {
        Golden {
            scene: dir.join("scene").to_string_lossy().into_owned(),
            tier,
        }
    }

    #[test]
    fn an_identical_frame_matches_and_a_missing_golden_differs() {
        let dir = tempfile::tempdir().expect("temp dir");
        let golden = golden_in(dir.path(), GoldenTier::Exact);
        let frame = flat(4, 4, 10);
        let out = dir.path().join("out");
        assert!(matches!(
            compare(&golden, &frame, &out, false),
            GoldenOutcome::Differs { .. }
        ));
        assert_eq!(compare(&golden, &frame, &out, true), GoldenOutcome::Blessed);
        assert_eq!(compare(&golden, &frame, &out, false), GoldenOutcome::Match);
    }

    #[test]
    fn the_exact_tier_refuses_one_step_and_the_soft_tier_allows_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let exact = golden_in(dir.path(), GoldenTier::Exact);
        let out = dir.path().join("out");
        assert_eq!(
            compare(&exact, &flat(10, 10, 10), &out, true),
            GoldenOutcome::Blessed
        );
        let mut one_off = flat(10, 10, 10);
        one_off.put_pixel(3, 3, Rgba([12, 10, 10, 255]));
        let outcome = compare(&exact, &one_off, &out, false);
        let GoldenOutcome::Differs { actual, diff } = outcome else {
            panic!("one pixel two steps off must differ in the exact tier");
        };
        assert!(actual.exists() && diff.exists());
        let soft = Golden {
            tier: GoldenTier::Soft {
                channel: 2,
                parts_per_million: 1000,
            },
            ..exact
        };
        assert_eq!(compare(&soft, &one_off, &out, false), GoldenOutcome::Match);
    }

    #[test]
    fn the_soft_tier_counts_pixels_beyond_the_channel_allowance() {
        let dir = tempfile::tempdir().expect("temp dir");
        let soft = golden_in(
            dir.path(),
            GoldenTier::Soft {
                channel: 2,
                parts_per_million: 1000,
            },
        );
        let out = dir.path().join("out");
        assert_eq!(
            compare(&soft, &flat(100, 100, 50), &out, true),
            GoldenOutcome::Blessed
        );
        // 10 of 10,000 pixels (1000 ppm) three steps off: within; 11 is not.
        let off = |n: u32| {
            (0..n).fold(flat(100, 100, 50), |mut image, i| {
                image.put_pixel(i, 0, Rgba([53, 50, 50, 255]));
                image
            })
        };
        assert_eq!(compare(&soft, &off(10), &out, false), GoldenOutcome::Match);
        assert!(matches!(
            compare(&soft, &off(11), &out, false),
            GoldenOutcome::Differs { .. }
        ));
    }

    #[test]
    fn a_frame_of_another_size_differs() {
        let dir = tempfile::tempdir().expect("temp dir");
        let golden = golden_in(dir.path(), GoldenTier::Exact);
        let out = dir.path().join("out");
        assert_eq!(
            compare(&golden, &flat(4, 4, 1), &out, true),
            GoldenOutcome::Blessed
        );
        assert!(matches!(
            compare(&golden, &flat(4, 5, 1), &out, false),
            GoldenOutcome::Differs { .. }
        ));
    }
}
