use super::super::slices::{Piece, slices};
use super::*;
use casement_core::geometry::LogicalSize;
use casement_core::window::FrameColour;

const BLACK: FrameColour = FrameColour {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

fn layer(dx: i32, dy: i32, blur: u32, spread: i32, a: u8) -> ShadowLayer {
    ShadowLayer {
        dx,
        dy,
        blur,
        spread,
        colour: FrameColour { a, ..BLACK },
    }
}

/// Like quire's window shadow: a tight layer and a wide one, both falling.
fn quire_like() -> [ShadowLayer; 2] {
    [layer(0, 2, 6, 0, 90), layer(0, 10, 24, -2, 70)]
}

fn alpha(r: &Raster, x: u32, y: u32) -> u8 {
    r.argb[((y * r.w + x) * 4 + 3) as usize]
}

fn pixel(r: &Raster, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * r.w + x) * 4) as usize;
    [r.argb[at], r.argb[at + 1], r.argb[at + 2], r.argb[at + 3]]
}

#[test]
fn a_corner_is_the_size_of_the_margin_and_reach_at_the_scale() {
    let layers = quire_like();
    for (scale, per) in [(Scale::ONE, 1), (Scale(240), 2)] {
        let made = tiles(&layers, 10, scale);
        let want = (MARGIN + made.inner) * per;
        for corner in &made.corners {
            assert_eq!((corner.w, corner.h), (want, want), "per {per}");
            assert_eq!(corner.argb.len() as u32, want * want * 4);
        }
        let strip = (MARGIN + EDGE_DEPTH) * per;
        assert_eq!((made.edges[0].w, made.edges[0].h), (STRIP_ACROSS, strip));
        assert_eq!((made.edges[2].w, made.edges[2].h), (strip, STRIP_ACROSS));
    }
}

#[test]
fn the_tiles_hold_no_window_sized_buffer() {
    // The function takes no window size; the whole set is bounded by the margin, the reach and
    // the scale.
    let made = tiles(&quire_like(), 10, Scale(240));
    let bytes: usize = made
        .corners
        .iter()
        .chain(&made.edges)
        .map(|r| r.argb.len())
        .sum();
    let side = 2 * (MARGIN + made.inner) as usize;
    assert!(
        bytes <= 4 * side * side * 4 + 4 * STRIP_ACROSS as usize * 2 * 2 * 100 * 4,
        "{bytes}"
    );
}

#[test]
fn without_an_offset_the_corners_are_mirror_images() {
    let layers = [layer(0, 0, 12, 0, 120)];
    let made = tiles(&layers, 10, Scale::ONE);
    let [tl, tr, bl, br] = &made.corners;
    let side = tl.w;
    for (x, y) in [(3, 9), (40, 41), (side - 1, 0), (80, 80)] {
        assert_eq!(pixel(tl, x, y), pixel(tr, side - 1 - x, y));
        assert_eq!(pixel(tl, x, y), pixel(bl, x, side - 1 - y));
        assert_eq!(pixel(tl, x, y), pixel(br, side - 1 - x, side - 1 - y));
    }
}

#[test]
fn a_falling_shadow_is_stronger_below_than_above() {
    let made = tiles(&quire_like(), 10, Scale::ONE);
    let [tl, _, bl, _] = &made.corners;
    let near = MARGIN - 6;
    assert!(alpha(bl, 100, near) > alpha(tl, 100, near));
}

#[test]
fn an_edge_strip_does_not_change_along_its_axis_and_meets_the_corner() {
    let made = tiles(&quire_like(), 10, Scale::ONE);
    // The strip is the same pixels along its length by construction; the corner's column at its
    // inner end is the same profile (no seam where they meet).
    let top = &made.edges[0];
    let corner = &made.corners[0];
    let x = corner.w - 1;
    for y in 0..top.h {
        let (a, b) = (alpha(top, 0, y), alpha(corner, x, y));
        assert!(a.abs_diff(b) <= 2, "row {y}: strip {a}, corner {b}");
    }
    let left = &made.edges[2];
    let y = corner.h - 1;
    for x in 0..left.w {
        let (a, b) = (alpha(left, x, 0), alpha(corner, x, y));
        assert!(a.abs_diff(b) <= 2, "col {x}: strip {a}, corner {b}");
    }
}

#[test]
fn a_rounded_window_has_less_shadow_at_its_corner_than_a_square_one() {
    let layers = [layer(0, 0, 12, 0, 200)];
    let square = tiles(&layers, 0, Scale::ONE);
    let round = tiles(&layers, 20, Scale::ONE);
    // Three pixels off the outline's corner on the diagonal: a rounded shape has drawn back from
    // the corner, a square one has not.
    let off = MARGIN - 3;
    assert!(alpha(&round.corners[0], off, off) < alpha(&square.corners[0], off, off));
    // Far along the straight edge they agree.
    let far = MARGIN + 60;
    assert!(alpha(&round.corners[0], far, 40).abs_diff(alpha(&square.corners[0], far, 40)) <= 2);
}

#[test]
fn the_tiles_are_sharp_at_scale_two() {
    // A narrow blur: at two device pixels to the logical pixel the edge is resolved to a
    // quarter pixel, so the pixels outside the edge are sampled at half the distance. The
    // pixel at half a logical pixel out (scale one) is between the two at scale two.
    let layers = [layer(0, 0, 2, 0, 255)];
    let one = tiles(&layers, 0, Scale::ONE);
    let two = tiles(&layers, 0, Scale(240));
    let outside_one = alpha(&one.edges[0], 0, MARGIN - 1);
    let near_two = alpha(&two.edges[0], 0, 2 * MARGIN - 1);
    let far_two = alpha(&two.edges[0], 0, 2 * MARGIN - 2);
    assert!(near_two > outside_one && outside_one > far_two);
    assert_eq!(two.edges[0].h, 2 * one.edges[0].h);
}

/// Table: the tiles are transparent under the window and carry the shadow outside it. Pixel
/// positions are (tile x, tile y) at scale one, with the window's corner at (MARGIN, MARGIN).
#[test]
fn the_shadow_is_cut_out_under_the_windows_shape() {
    let layers = [layer(0, 0, 12, 0, 200)];
    let m = MARGIN;
    for (scale, per) in [(Scale::ONE, 1), (Scale(240), 2)] {
        let made = tiles(&layers, 20, scale);
        let corner = &made.corners[0];
        let px = |logical: u32| logical * per;
        // The far inside edge of the tile (its last column or row), and a point well inside.
        let far = made.inner - 1;
        let deep = far.min(30);
        // name, corner tile pixel, expected nonzero
        let table = [
            ("deep inside the window", (m + deep, m + deep), false),
            (
                "inside, near the straight top edge",
                (m + far, m + 2),
                false,
            ),
            (
                "inside, near the straight left edge",
                (m + 2, m + far),
                false,
            ),
            ("on the curve's inside, 6 px in", (m + 10, m + 10), false),
            ("in the square, outside the curve", (m + 1, m + 1), true),
            ("just outside the top edge", (m + far, m - 3), true),
            ("just outside the left edge", (m - 3, m + far), true),
            ("out beyond the corner", (m - 3, m - 3), true),
        ];
        for (name, (x, y), shadowed) in table {
            let got = alpha(corner, px(x), px(y));
            assert_eq!(got > 0, shadowed, "{name} at scale {per}: alpha {got}");
        }
        // The strips are cut the same way: inside the edge is empty, outside has shadow.
        let top = &made.edges[0];
        assert_eq!(
            alpha(top, 0, px(m + 1)),
            0,
            "strip under the edge, scale {per}"
        );
        assert!(
            alpha(top, 0, px(m - 3)) > 0,
            "strip outside the edge, scale {per}"
        );
        // A right strip reads outwards to the right: its first pixel is the deepest inside.
        let right = &made.edges[3];
        assert_eq!(alpha(right, 0, 0), 0, "right strip under the edge");
        assert!(
            alpha(right, right.w - 1 - px(m - 3), 0) > 0,
            "right strip outside"
        );
    }
}

/// A pixel straddling the window's edge keeps part of its shadow (the window's own anti-aliased
/// edge pixel is blended over it), and the cut is complete a pixel in.
#[test]
fn the_cut_out_starts_half_a_pixel_inside_the_edge() {
    let layers = [layer(0, 0, 4, 0, 255)];
    let made = tiles(&layers, 0, Scale::ONE);
    let top = &made.edges[0];
    let m = MARGIN;
    let (outside, edge, inside) = (alpha(top, 0, m - 1), alpha(top, 0, m), alpha(top, 0, m + 1));
    assert!(outside > edge && edge > inside, "{outside} {edge} {inside}");
    assert_eq!(inside, 0);
}

/// The old full raster of a square-cornered shadow at one pixel to a logical pixel.
fn reference(layers: &[ShadowLayer], width: u32, height: u32) -> Raster {
    let margin = MARGIN as f32;
    let (w, h) = (width + 2 * MARGIN, height + 2 * MARGIN);
    let bar = |at: f32, low: f32, high: f32, s: f32| half(at, low, s) - half(at, high, s);
    let argb = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .flat_map(|(x, y)| {
            let mut px: Px = [0.0; 4];
            for l in layers {
                let s = sigma(l);
                let g = l.spread as f32;
                let (left, top) = (margin + l.dx as f32 - g, margin + l.dy as f32 - g);
                let across = bar(x as f32 + 0.5, left, left + width as f32 + 2.0 * g, s);
                let down = bar(y as f32 + 0.5, top, top + height as f32 + 2.0 * g, s);
                over(&mut px, l.colour, across * down);
            }
            // The window's own square shape cut out, as the tiles do.
            let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
            let inside = (margin - cx)
                .max(cx - margin - width as f32)
                .max(margin - cy)
                .max(cy - margin - height as f32);
            bytes(punch(px, inside, 1.0))
        })
        .collect();
    Raster { w, h, argb }
}

/// Every pixel the pieces draw for a window `width` by `height` at scale one, with its place in
/// the old raster.
fn assembled(made: &Tiles, width: u32, height: u32) -> Vec<((u32, u32), [u8; 4])> {
    let mut out = Vec::new();
    for s in slices(
        LogicalSize {
            w: width,
            h: height,
        },
        made.inner,
    ) {
        let tile = match s.piece {
            Piece::Corner(c) => &made.corners[crate::element::corner::corner_index(c)],
            Piece::Edge(e) => &made.edges[e.index()],
        };
        for dy in 0..s.dest.size.h {
            for dx in 0..s.dest.size.w {
                let along = |o: i32, d: u32, len: u32| match len == STRIP_ACROSS {
                    true => STRIP_PAD,
                    false => o as u32 + d,
                };
                let (sx, sy) = (
                    along(s.src.origin.x, dx, tile.w),
                    along(s.src.origin.y, dy, tile.h),
                );
                let place = (
                    (s.dest.origin.x + dx as i32 + MARGIN as i32) as u32,
                    (s.dest.origin.y + dy as i32 + MARGIN as i32) as u32,
                );
                out.push((place, pixel(tile, sx, sy)));
            }
        }
    }
    out
}

#[test]
fn the_pieces_add_up_to_the_old_raster_of_a_square_window() {
    let layers = [layer(0, 2, 8, 0, 90), layer(3, 10, 20, -2, 70)];
    let made = tiles(&layers, 0, Scale::ONE);
    for (w, h) in [(300, 200), (400, 90), (150, 150), (80, 80)] {
        let whole = reference(&layers, w, h);
        let drawn = assembled(&made, w, h);
        assert!(drawn.len() > 1000, "{w}x{h}");
        // A window smaller than twice the reach crops its corners, and its short blur differs
        // by design; the others must match.
        let exact = w >= 2 * made.inner && h >= 2 * made.inner;
        let worst = drawn
            .iter()
            .map(|((x, y), got)| {
                let want = pixel(&whole, *x, *y);
                got.iter()
                    .zip(want)
                    .map(|(a, b)| a.abs_diff(b))
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);
        let limit = if exact { 4 } else { 40 };
        assert!(worst <= limit, "{w}x{h}: off by {worst}");
    }
}

/// A window `w` by `h` as one raster and as the tiles' pieces draw it: the same shadow, up to
/// the pieces' own rounding.
#[test]
fn the_whole_raster_agrees_with_the_pieces_of_a_large_window() {
    let layers = quire_like();
    let made = tiles(&layers, 10, Scale::ONE);
    let (w, h) = (300, 200);
    let whole = full(&layers, 10, LogicalSize { w, h }, Scale::ONE);
    assert_eq!((whole.w, whole.h), (w + 2 * MARGIN, h + 2 * MARGIN));
    let drawn = assembled(&made, w, h);
    assert!(drawn.len() > 10_000);
    let worst = drawn
        .iter()
        .map(|((x, y), got)| {
            let want = pixel(&whole, *x, *y);
            got.iter()
                .zip(want)
                .map(|(a, b)| a.abs_diff(b))
                .max()
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0);
    assert!(worst <= 4, "off by {worst}");
}

/// A tiny window's cropped pieces shadow it more strongly than the blur of its shape, which the
/// whole raster is; below its bottom edge the whole raster is the lighter one.
#[test]
fn a_tiny_windows_whole_raster_is_lighter_than_its_cropped_tiles() {
    let layers = [layer(0, 4, 24, 0, 120)];
    let made = tiles(&layers, 10, Scale::ONE);
    let size = LogicalSize { w: 24, h: 24 };
    assert!(wanted(size, made.inner));
    let whole = full(&layers, 10, size, Scale::ONE);
    let drawn = assembled(&made, 24, 24);
    // Six pixels below the bottom edge, in the middle.
    let at = (MARGIN + 12, MARGIN + 24 + 6);
    let cropped = drawn
        .iter()
        .find(|(place, _)| *place == at)
        .map(|(_, p)| p[3]);
    let true_blur = pixel(&whole, at.0, at.1)[3];
    let cropped = cropped.expect("the pieces cover the point");
    assert!(
        cropped > true_blur + 4,
        "cropped {cropped}, whole {true_blur}"
    );
}

#[test]
fn a_whole_raster_is_cut_out_and_sized_to_its_window_at_each_scale() {
    let layers = quire_like();
    let size = LogicalSize { w: 90, h: 60 };
    for (scale, per) in [(Scale::ONE, 1.0_f32), (Scale(180), 1.5), (Scale(240), 2.0)] {
        let r = full(&layers, 10, size, scale);
        let pixels = |n: u32| ((n + 2 * MARGIN) as f32 * per).round() as u32;
        assert_eq!((r.w, r.h), (pixels(90), pixels(60)), "scale {per}");
        assert_eq!(
            alpha(&r, r.w / 2, r.h / 2),
            0,
            "under the window, scale {per}"
        );
        // Six logical pixels below the bottom edge, in the middle: shadowed.
        let below = ((MARGIN + 60 + 6) as f32 * per) as u32;
        assert!(
            alpha(&r, r.w / 2, below) > 0,
            "below the window, scale {per}"
        );
    }
}

#[test]
fn which_windows_get_a_raster_of_their_own() {
    let inner = 59;
    for (w, h, want) in [
        (100, 60, true),
        (117, 400, false),
        (117, 200, true),
        (400, 300, false),
        (118, 118, false),
        (117, 117, true),
        (240, 100, true),
        (241, 100, false),
        (60, 1000, false),
    ] {
        assert_eq!(wanted(LogicalSize { w, h }, inner), want, "{w}x{h}");
    }
}
