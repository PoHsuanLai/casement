//! The icon of an app that has none: a plain rounded plate.

use crate::Pixels;
use std::sync::Arc;

/// How round the corners are, in parts of a thousand of the edge.
const RADIUS: f32 = 225.0;

/// A rounded square of `ink` (straight R, G, B, A) filling `edge` by `edge` pixels, its edge
/// smoothed over one pixel. Nothing for an edge of zero.
///
/// ```
/// use casement_icons::plate;
///
/// let got = plate(16, [128, 128, 128, 255]).unwrap();
/// assert_eq!(got.rgba.len(), 16 * 16 * 4);
/// assert_eq!(&got.rgba[(8 * 16 + 8) * 4..][..4], &[128, 128, 128, 255]);
/// assert_eq!(got.rgba[3], 0); // the corner is clear
/// ```
pub fn plate(edge: u32, ink: [u8; 4]) -> Option<Pixels> {
    (edge > 0).then(|| {
        let side = edge as f32;
        let radius = side * RADIUS / 1000.0;
        let rgba: Arc<[u8]> = (0..edge)
            .flat_map(|y| (0..edge).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let cover = coverage((x as f32 + 0.5, y as f32 + 0.5), side, radius);
                let alpha = f32::from(ink[3]) * cover;
                let part = |c: u8| (f32::from(c) * alpha / 255.0).round() as u8;
                [
                    part(ink[0]),
                    part(ink[1]),
                    part(ink[2]),
                    alpha.round() as u8,
                ]
            })
            .collect();
        Pixels { edge, rgba }
    })
}

/// How much of the pixel centred at `at` the rounded square of `side` and `radius` covers, 0 to 1.
fn coverage((x, y): (f32, f32), side: f32, radius: f32) -> f32 {
    let half = side / 2.0;
    let (dx, dy) = (
        (x - half).abs() - (half - radius),
        (y - half).abs() - (half - radius),
    );
    let outside = dx.max(0.0).hypot(dy.max(0.0));
    let inside = dx.max(dy).min(0.0);
    (0.5 - (outside + inside - radius)).clamp(0.0, 1.0)
}
