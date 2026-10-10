//! The writeback comparison (P1-V): pure, no device. Raw buffers as the vkms writeback
//! connector hands them back, decoded and held to a golden under a per-channel tolerance.

use casement_render::golden::{GoldenOutcome, GoldenTier};
use casement_testkit::writeback::{
    PixelFormat, RawFrame, WritebackError, compare_raw, decode, tolerance,
};
use image::{Rgba, RgbaImage};

/// A 2x2 golden of flat grey at `dir/grey`.
fn grey_golden(dir: &std::path::Path) -> String {
    let scene = dir.join("grey").to_string_lossy().into_owned();
    RgbaImage::from_pixel(2, 2, Rgba([100, 100, 100, 255]))
        .save(format!("{scene}.png"))
        .expect("golden");
    scene
}

/// 2x2 XRGB8888 pixels of `grey`, rows padded to a pitch of 12 bytes.
fn grey_raw(grey: u8) -> Vec<u8> {
    let row = [grey, grey, grey, 0, grey, grey, grey, 0, 7, 7, 7, 7];
    [row, row].concat()
}

fn frame() -> RawFrame {
    RawFrame::new(2, 2, 12, PixelFormat::Xrgb8888)
}

#[test]
fn decode_swaps_blue_first_to_rgba_and_skips_row_padding() {
    let bytes = [
        3, 2, 1, 0, 6, 5, 4, 0, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    ];
    let image = decode(&RawFrame::new(2, 2, 12, PixelFormat::Xrgb8888), &bytes[..]);
    let image = image.expect("a frame");
    assert_eq!(image.get_pixel(0, 0).0, [1, 2, 3, 255]);
    assert_eq!(image.get_pixel(1, 0).0, [4, 5, 6, 255]);
}

#[test]
fn argb_keeps_its_alpha_and_xrgb_ignores_the_fourth_byte() {
    let bytes = [3, 2, 1, 40];
    let argb = decode(&RawFrame::new(1, 1, 4, PixelFormat::Argb8888), &bytes).expect("argb");
    let xrgb = decode(&RawFrame::new(1, 1, 4, PixelFormat::Xrgb8888), &bytes).expect("xrgb");
    assert_eq!(argb.get_pixel(0, 0).0[3], 40);
    assert_eq!(xrgb.get_pixel(0, 0).0[3], 255);
}

#[test]
fn a_malformed_buffer_is_refused_not_misread() {
    let short = decode(&frame(), &[0; 10]);
    assert!(matches!(short, Err(WritebackError::Short { have: 10, .. })));
    let narrow = decode(&RawFrame::new(2, 2, 4, PixelFormat::Xrgb8888), &[0; 64]);
    assert!(matches!(narrow, Err(WritebackError::PitchTooSmall { .. })));
    let empty = decode(&RawFrame::new(0, 2, 4, PixelFormat::Xrgb8888), &[]);
    assert_eq!(empty, Err(WritebackError::Empty));
}

#[test]
fn the_last_row_needs_no_trailing_padding() {
    // pitch 12, height 2, width 2: 12 + 8 = 20 bytes are enough.
    let image = decode(&frame(), &grey_raw(100)[..20]);
    assert!(image.is_ok());
}

#[test]
fn a_frame_within_the_tolerance_matches_and_one_beyond_it_does_not() {
    let dir = tempfile::tempdir().expect("temp dir");
    let scene = grey_golden(dir.path());
    let out = dir.path().join("out");
    let tier = tolerance(3, 0);
    let near = compare_raw(&frame(), &grey_raw(103), &scene, tier, &out);
    assert_eq!(near, Ok(GoldenOutcome::Match));
    let far = compare_raw(&frame(), &grey_raw(104), &scene, tier, &out);
    assert!(matches!(far, Ok(GoldenOutcome::Differs { .. })));
    assert!(
        out.join("grey.actual.png").exists(),
        "the frame is kept for a look"
    );
}

#[test]
fn a_few_stray_pixels_pass_only_with_a_parts_per_million_allowance() {
    let dir = tempfile::tempdir().expect("temp dir");
    let scene = grey_golden(dir.path());
    let out = dir.path().join("out");
    let mut bytes = grey_raw(100);
    bytes[0] = 200;
    let strict = compare_raw(&frame(), &bytes, &scene, tolerance(3, 0), &out);
    assert!(matches!(strict, Ok(GoldenOutcome::Differs { .. })));
    let loose = compare_raw(&frame(), &bytes, &scene, tolerance(3, 300_000), &out);
    assert_eq!(loose, Ok(GoldenOutcome::Match));
}

#[test]
fn a_size_other_than_the_goldens_differs() {
    let dir = tempfile::tempdir().expect("temp dir");
    let scene = grey_golden(dir.path());
    let one = RawFrame::new(1, 1, 4, PixelFormat::Xrgb8888);
    let outcome = compare_raw(
        &one,
        &[100, 100, 100, 0],
        &scene,
        GoldenTier::Exact,
        dir.path(),
    );
    assert!(matches!(outcome, Ok(GoldenOutcome::Differs { .. })));
}

#[test]
fn format_names_are_the_drm_fourccs() {
    assert_eq!(PixelFormat::named("AR24"), Some(PixelFormat::Argb8888));
    assert_eq!(PixelFormat::named("RG16"), None);
}
