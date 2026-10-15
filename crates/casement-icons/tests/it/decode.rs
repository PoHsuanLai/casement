use crate::support::png;
use casement_icons::{load, plate};

const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="48" height="24" viewBox="0 0 48 24"><rect width="48" height="24" fill="#ff0000"/></svg>"##;

fn at(rgba: &[u8], edge: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * edge + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

#[test]
fn a_png_is_scaled_to_the_exact_edge() {
    let dir = tempfile::tempdir().expect("scratch");
    let file = dir.path().join("a.png");
    png(&file, 20, [0, 255, 0, 255]);
    for edge in [10, 20, 64, 129] {
        let got = load(&file, edge).expect("decodes");
        assert_eq!(
            (got.edge, got.rgba.len()),
            (edge, (edge * edge * 4) as usize)
        );
        assert_eq!(
            at(&got.rgba, edge, edge / 2, edge / 2),
            [0, 255, 0, 255],
            "{edge}"
        );
    }
}

#[test]
fn a_png_with_alpha_comes_out_premultiplied() {
    let dir = tempfile::tempdir().expect("scratch");
    let file = dir.path().join("a.png");
    png(&file, 4, [200, 100, 0, 128]);
    let got = load(&file, 4).expect("decodes");
    assert_eq!(at(&got.rgba, 4, 1, 1), [100, 50, 0, 128]);
}

#[test]
fn an_svg_fills_the_edge_in_proportion() {
    let dir = tempfile::tempdir().expect("scratch");
    let file = dir.path().join("wide.svg");
    std::fs::write(&file, SVG).expect("write");
    let got = load(&file, 48).expect("draws");
    assert_eq!(got.edge, 48);
    // 48 by 24 drawn at full width, centred: clear bands above and below.
    assert_eq!(at(&got.rgba, 48, 24, 24), [255, 0, 0, 255]);
    assert_eq!(at(&got.rgba, 48, 24, 2)[3], 0);
    assert_eq!(at(&got.rgba, 48, 24, 45)[3], 0);
}

#[test]
fn what_is_not_a_picture_or_not_there_is_an_error() {
    let dir = tempfile::tempdir().expect("scratch");
    let junk = dir.path().join("junk.png");
    std::fs::write(&junk, b"not a picture").expect("write");
    assert!(load(&junk, 32).is_err());
    assert!(load(&dir.path().join("absent.png"), 32).is_err());
    assert!(load(&junk, 0).is_err());
}

#[test]
fn the_plate_is_round_cornered_and_opaque_inside() {
    assert!(plate(0, [0, 0, 0, 255]).is_none());
    let got = plate(32, [128, 128, 128, 255]).expect("a plate");
    assert_eq!(at(&got.rgba, 32, 16, 16), [128, 128, 128, 255]);
    assert_eq!(at(&got.rgba, 32, 0, 0)[3], 0);
    assert_eq!(at(&got.rgba, 32, 16, 0)[3], 255);
}
