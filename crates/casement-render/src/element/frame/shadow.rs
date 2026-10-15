//! The window shadow's rasters: four corner tiles and four edge strips, a pure function of the
//! shadow's layers, the window's corner radius and the output's scale. The size of the window
//! is not an input, so a window being resized draws the same tiles every frame (`slices.rs`
//! says where they go).
//!
//! Each layer is the outline's rounded rectangle moved by `dx`, `dy`, grown by `spread` and
//! blurred (sigma half the blur radius). A corner tile is the blurred corner of that shape,
//! made by a separable Gaussian over its exact coverage, so the blur follows the rounded shape
//! and not a square. An edge strip is the blurred straight edge, a one-dimensional profile that
//! is constant along the edge. Tiles are made at the output's scale, so they are sharp.
//!
//! The shadow is cut out under the window: where the window's rounded shape is, the tiles are
//! transparent (`punch`), so a translucent window does not show its own shadow through itself.
//! A window too small for the tiles to meet has one raster of its own size (`whole.rs`).

use super::raster::{Px, Raster, bytes, cover, over};
use super::slices::{Corner, EDGE_DEPTH, MARGIN, Side};
use casement_core::window::ShadowLayer;
use ds_core::geometry::scale::Scale;

/// What a shadow is drawn from: its corner tiles (in `Corner::ALL` order) and edge strips (in
/// `Side::ALL` order), and how far the corners reach into the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tiles {
    /// How far a corner tile reaches into the window, in logical pixels.
    pub(crate) inner: u32,
    /// The four corners, each `MARGIN + inner` logical pixels square.
    pub(crate) corners: [Raster; 4],
    /// The four strips, one pixel across and `MARGIN + EDGE_DEPTH` logical pixels long.
    pub(crate) edges: [Raster; 4],
}

/// The sigma of a layer's blur, in logical pixels.
fn sigma(layer: &ShadowLayer) -> f32 {
    (layer.blur as f32 / 2.0).max(0.5)
}

/// How far into the window a corner tile must reach for the straight edge to take over from it
/// without a seam: past the curve and the blur, from where the layers' shapes start.
pub(crate) fn inner_reach(layers: &[ShadowLayer], radius: u32) -> u32 {
    layers
        .iter()
        .map(|l| {
            radius
                + (3.0 * sigma(l)).ceil() as u32
                + l.dx.unsigned_abs().max(l.dy.unsigned_abs())
                + l.spread.unsigned_abs()
                + 1
        })
        .max()
        .unwrap_or(radius + 1)
        .min(2 * MARGIN)
}

/// The tiles of a shadow made of `layers` (first drawn first) round a window whose corners have
/// `radius`, at `scale`.
pub(crate) fn tiles(layers: &[ShadowLayer], radius: u32, scale: Scale) -> Tiles {
    let per = scale.numerator() as f32 / Scale::DENOMINATOR as f32;
    let inner = inner_reach(layers, radius);
    let mut made: Vec<(Vec<(i32, i32)>, Raster)> = Vec::new();
    let corners = Corner::ALL.map(|corner| {
        let (sx, sy) = corner.sign();
        let offsets: Vec<(i32, i32)> = layers.iter().map(|l| (sx * l.dx, sy * l.dy)).collect();
        let found = made.iter().position(|(key, _)| *key == offsets);
        let at = found.unwrap_or_else(|| {
            let raster = corner_local(layers, &offsets, (radius, inner), per);
            made.push((offsets, raster));
            made.len() - 1
        });
        mirrored(&made[at].1, (sx, sy))
    });
    let edges = Side::ALL.map(|side| strip(layers, side, per));
    Tiles {
        inner,
        corners,
        edges,
    }
}

/// How many pixels `logical` is at `per` pixels to a logical pixel, at least one.
fn pixels(logical: u32, per: f32) -> usize {
    ((logical as f32 * per).round() as usize).max(1)
}

/// The corner tile in the corner's own frame (`u` and `v` grow inward from the tile's outer
/// corner), for layers moved by `offsets`.
fn corner_local(
    layers: &[ShadowLayer],
    offsets: &[(i32, i32)],
    (radius, inner): (u32, u32),
    per: f32,
) -> Raster {
    let side = pixels(MARGIN + inner, per);
    let fields: Vec<Vec<f32>> = layers
        .iter()
        .zip(offsets)
        .map(|(layer, off)| field(layer, *off, radius, per, side))
        .collect();
    let at = |i: usize| (i as f32 + 0.5) / per - MARGIN as f32;
    let argb = (0..side * side)
        .flat_map(|i| {
            let mut px: Px = [0.0; 4];
            for (layer, f) in layers.iter().zip(&fields) {
                over(&mut px, layer.colour, f[i]);
            }
            let inside = distance((at(i % side), at(i / side)), (0.0, 0.0), radius as f32);
            bytes(punch(px, inside, per))
        })
        .collect();
    Raster {
        w: side as u32,
        h: side as u32,
        argb,
    }
}

/// `raster` flipped across the axes `sign` says are mirrored.
fn mirrored(raster: &Raster, sign: (i32, i32)) -> Raster {
    let (w, h) = (raster.w as usize, raster.h as usize);
    let argb = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .flat_map(|(x, y)| {
            let sx = if sign.0 < 0 { w - 1 - x } else { x };
            let sy = if sign.1 < 0 { h - 1 - y } else { y };
            let at = (sy * w + sx) * 4;
            raster.argb[at..at + 4].iter().copied()
        })
        .collect();
    Raster {
        w: raster.w,
        h: raster.h,
        argb,
    }
}

/// The signed distance from `p` to the corner of a rounded rectangle whose straight edges are
/// at `edge` (the shape lies to the right of and below them) with corner `round`.
fn distance(p: (f32, f32), edge: (f32, f32), round: f32) -> f32 {
    let centre = (edge.0 + round, edge.1 + round);
    if p.0 < centre.0 && p.1 < centre.1 {
        (p.0 - centre.0).hypot(p.1 - centre.1) - round
    } else {
        (edge.0 - p.0).max(edge.1 - p.1)
    }
}

/// A normalised Gaussian over `2 * pad + 1` taps.
fn kernel(sigma: f32, pad: usize) -> Vec<f32> {
    let raw: Vec<f32> = (0..=2 * pad)
        .map(|k| {
            let t = k as f32 - pad as f32;
            (-t * t / (2.0 * sigma * sigma)).exp()
        })
        .collect();
    let total: f32 = raw.iter().sum();
    raw.iter().map(|w| w / total).collect()
}

/// One layer's blurred alpha over a `side` square corner tile: the exact coverage of the
/// rounded corner, blurred along rows and then along columns. Only the tile and the rows the
/// blur reads from are ever held, never a window.
fn field(layer: &ShadowLayer, off: (i32, i32), radius: u32, per: f32, side: usize) -> Vec<f32> {
    let grow = layer.spread as f32;
    let edge = (off.0 as f32 - grow, off.1 as f32 - grow);
    let round = (radius as f32 + grow).max(0.0);
    blurred(layer, (side, side), per, |p| distance(p, edge, round))
}

/// One layer's alpha over a `cols` by `rows` grid whose top-left is `MARGIN` logical pixels
/// above and left of the outline's corner: the shape `dist` says (a signed distance from a
/// point in logical pixels from the outline's corner, negative inside), blurred along rows and
/// then along columns. The coverage is computed over a margin of three sigma round the grid so
/// the blur has no edge.
fn blurred(
    layer: &ShadowLayer,
    (cols, rows): (usize, usize),
    per: f32,
    dist: impl Fn((f32, f32)) -> f32,
) -> Vec<f32> {
    let sigma_px = sigma(layer) * per;
    let pad = (3.0 * sigma_px).ceil() as usize;
    let weights = kernel(sigma_px, pad);
    let (span_x, span_y) = (cols + 2 * pad, rows + 2 * pad);
    let at = |i: usize| (i as f32 - pad as f32 + 0.5) / per - MARGIN as f32;
    let mut across = vec![0.0f32; span_y * cols];
    for r in 0..span_y {
        let v = at(r);
        let row: Vec<f32> = (0..span_x).map(|c| cover(dist((at(c), v)), per)).collect();
        for x in 0..cols {
            across[r * cols + x] = row[x..x + weights.len()]
                .iter()
                .zip(&weights)
                .map(|(a, b)| a * b)
                .sum();
        }
    }
    let mut out = vec![0.0f32; cols * rows];
    for y in 0..rows {
        for (k, w) in weights.iter().enumerate() {
            let source = &across[(y + k) * cols..][..cols];
            for (o, s) in out[y * cols..][..cols].iter_mut().zip(source) {
                *o += s * w;
            }
        }
    }
    out
}

/// `px` with the window's shape cut out of it: `inside` is the distance in logical pixels from
/// the pixel's centre to the window's edge (negative under the window). The cut starts half a
/// device pixel inside the edge, so the window's own anti-aliased edge pixel still has its
/// shadow under it, and is complete one device pixel in.
fn punch(px: Px, inside: f32, per: f32) -> Px {
    let keep = 1.0 - cover(inside + 0.5 / per, per);
    px.map(|v| v * keep)
}

/// The error function (Abramowitz and Stegun 7.1.26, error under 1.5e-7).
fn erf(x: f32) -> f32 {
    let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
    let poly = t
        * (0.254_829_6
            + t * (-0.284_496_74 + t * (1.421_413_8 + t * (-1.453_152_1 + t * 1.061_405_4))));
    let value = 1.0 - poly * (-x * x).exp();
    value.copysign(x)
}

/// The share of a blurred half-plane (sigma `sigma`) that starts at `low` which lies on `at`.
fn half(at: f32, low: f32, sigma: f32) -> f32 {
    0.5 * (1.0 + erf((at - low) / (sigma * std::f32::consts::SQRT_2)))
}

/// How far a layer falls towards `side`'s inside, in logical pixels.
fn along(layer: &ShadowLayer, side: Side) -> i32 {
    match side {
        Side::Top => layer.dy,
        Side::Bottom => -layer.dy,
        Side::Left => layer.dx,
        Side::Right => -layer.dx,
    }
}

/// How many pixels an edge strip is across its length: three identical ones. A strip is the same
/// along the whole edge, so one would do, but it is drawn stretched along the edge, and a
/// renderer that filters the stretch (pixman does, bilinearly, with nothing beyond the image's
/// own edge) fades the first and last half texel of a one-pixel strip, a seam where it meets the
/// corner tiles. The strip is drawn from the middle pixel alone ([`STRIP_PAD`] in from each end),
/// so the filter finds the same pixel on both sides of every pixel it draws.
pub(crate) const STRIP_ACROSS: u32 = 3;

/// How many pixels of an edge strip lie past each end of the part of it that is drawn.
pub(crate) const STRIP_PAD: u32 = (STRIP_ACROSS - 1) / 2;

/// The strip for `side`: the blurred straight edge from `MARGIN` outside to `EDGE_DEPTH` inside,
/// [`STRIP_ACROSS`] identical pixels wide. A top or left strip runs outside first; a bottom or
/// right one is the other way round, so every strip reads outwards in the direction of the screen.
fn strip(layers: &[ShadowLayer], side: Side, per: f32) -> Raster {
    let n = pixels(MARGIN + EDGE_DEPTH, per);
    let across = STRIP_ACROSS as usize;
    let line: Vec<[u8; 4]> = (0..n)
        .map(|j| {
            let i = match side {
                Side::Top | Side::Left => j,
                Side::Bottom | Side::Right => n - 1 - j,
            };
            let t = (i as f32 + 0.5) / per - MARGIN as f32;
            let mut px: Px = [0.0; 4];
            for layer in layers {
                let low = (along(layer, side) - layer.spread) as f32;
                over(&mut px, layer.colour, half(t, low, sigma(layer)));
            }
            bytes(punch(px, -t, per))
        })
        .collect();
    match side {
        Side::Top | Side::Bottom => Raster {
            w: STRIP_ACROSS,
            h: n as u32,
            argb: line
                .iter()
                .flat_map(|px| std::iter::repeat_n(*px, across).flatten())
                .collect(),
        },
        Side::Left | Side::Right => Raster {
            w: n as u32,
            h: STRIP_ACROSS,
            argb: (0..across)
                .flat_map(|_| line.iter().flatten().copied())
                .collect(),
        },
    }
}

mod whole;

pub(crate) use whole::{full, wanted};

#[cfg(test)]
mod tests;
