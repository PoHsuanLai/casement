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
        assert_eq!((made.edges[0].w, made.edges[0].h), (1, strip));
        assert_eq!((made.edges[2].w, made.edges[2].h), (strip, 1));
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
        bytes <= 4 * side * side * 4 + 4 * 2 * 2 * 100 * 4,
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
    // The strip is one pixel across by construction; the corner's column at its inner end is
    // the same profile (no seam where they meet).
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
    let at = MARGIN + 1;
    // Just inside the outline's corner: a square shape covers it, a rounded one does not.
    assert!(alpha(&round.corners[0], at, at) < alpha(&square.corners[0], at, at));
    // Far along the straight edge they agree.
    let far = MARGIN + 60;
    assert!(alpha(&round.corners[0], far, 40).abs_diff(alpha(&square.corners[0], far, 40)) <= 2);
}

#[test]
fn the_tiles_are_sharp_at_scale_two() {
    // A narrow blur: at two device pixels to the logical pixel the edge is resolved to a
    // quarter pixel, so the first pixel inside is less far up the ramp than at scale one, and
    // the two pixels either side of the edge still add up to one whole.
    let layers = [layer(0, 0, 2, 0, 255)];
    let one = tiles(&layers, 0, Scale::ONE);
    let two = tiles(&layers, 0, Scale(240));
    let inside_one = alpha(&one.edges[0], 0, MARGIN);
    let inside_two = alpha(&two.edges[0], 0, 2 * MARGIN);
    let outside_two = alpha(&two.edges[0], 0, 2 * MARGIN - 1);
    assert!(inside_two < inside_one, "{inside_two} {inside_one}");
    assert!(inside_two.abs_diff(255 - outside_two) <= 3);
    assert_eq!(two.edges[0].h, 2 * one.edges[0].h);
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
            bytes(px)
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
            Piece::Corner(c) => &made.corners[c.index()],
            Piece::Edge(e) => &made.edges[e.index()],
        };
        for dy in 0..s.dest.size.h {
            for dx in 0..s.dest.size.w {
                let along = |o: i32, d: u32, len: u32| if len == 1 { 0 } else { o as u32 + d };
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
