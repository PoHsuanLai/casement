//! The shadow of a window too small for the tiles: one raster of the window's own size (plus the
//! margin), the true blur of its rounded shape. The tiles assume a window big enough that no
//! corner sees the far side; a window under about 120 to 150 logical pixels is not, and its
//! cropped tiles would shadow it too strongly. Such windows are few and mostly still, so the
//! raster is cached by size (`DrawCache::whole_shadow`).

use super::{MARGIN, Px, Raster, blurred, bytes, over, pixels, punch};
use casement_core::geometry::LogicalSize;
use casement_core::window::ShadowLayer;
use ds_core::geometry::scale::Scale;

/// The most a window may measure on either side to be given a raster of its own, in logical
/// pixels (a bigger one is cheaper in tiles, and would take a long time to blur).
pub(crate) const LIMIT: u32 = 240;

/// Whether a window of `size`, whose corner tiles reach `inner` logical pixels in, is drawn from
/// a raster of its own: one side is under twice the reach (the tiles would overlap and crop), and
/// neither is past [`LIMIT`].
pub(crate) fn wanted(size: LogicalSize, inner: u32) -> bool {
    let small = size.w < 2 * inner || size.h < 2 * inner;
    small && size.w <= LIMIT && size.h <= LIMIT
}

/// The signed distance from `p` to the rounded rectangle `rect` (left, top, right, bottom) with
/// corners of `round`, negative inside.
fn rounded(p: (f32, f32), rect: (f32, f32, f32, f32), round: f32) -> f32 {
    let half = ((rect.2 - rect.0) / 2.0, (rect.3 - rect.1) / 2.0);
    let centre = (rect.0 + half.0, rect.1 + half.1);
    let r = round.min(half.0).min(half.1).max(0.0);
    let q = (
        (p.0 - centre.0).abs() - half.0 + r,
        (p.1 - centre.1).abs() - half.1 + r,
    );
    q.0.max(q.1).min(0.0) + q.0.max(0.0).hypot(q.1.max(0.0)) - r
}

/// The shadow of a window `size` with corners of `radius`, made of `layers`, at `scale`: a raster
/// covering the window and `MARGIN` logical pixels on every side, with the window's own shape
/// cut out. It is drawn at the size of the window plus twice the margin, from the outline's
/// corner minus the margin.
pub(crate) fn full(layers: &[ShadowLayer], radius: u32, size: LogicalSize, scale: Scale) -> Raster {
    let per = scale.numerator() as f32 / Scale::DENOMINATOR as f32;
    let (w, h) = (size.w as f32, size.h as f32);
    let cols = pixels(size.w + 2 * MARGIN, per);
    let rows = pixels(size.h + 2 * MARGIN, per);
    let fields: Vec<Vec<f32>> = layers
        .iter()
        .map(|layer| {
            let grow = layer.spread as f32;
            let (dx, dy) = (layer.dx as f32, layer.dy as f32);
            let rect = (dx - grow, dy - grow, w + dx + grow, h + dy + grow);
            let round = (radius as f32 + grow).max(0.0);
            blurred(layer, (cols, rows), per, |p| rounded(p, rect, round))
        })
        .collect();
    let window = (0.0, 0.0, w, h);
    let argb = (0..cols * rows)
        .flat_map(|i| {
            let mut px: Px = [0.0; 4];
            for (layer, f) in layers.iter().zip(&fields) {
                over(&mut px, layer.colour, f[i]);
            }
            let at = |n: usize| (n as f32 + 0.5) / per - MARGIN as f32;
            let inside = rounded((at(i % cols), at(i / cols)), window, radius as f32);
            bytes(punch(px, inside, per))
        })
        .collect();
    Raster {
        w: cols as u32,
        h: rows as u32,
        argb,
    }
}
