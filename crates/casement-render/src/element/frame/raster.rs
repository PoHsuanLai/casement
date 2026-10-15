//! The server-side frame's rasters: the titlebar with its rounded top corners and its three
//! lights (the window's shadow is `shadow.rs`). Pure functions of a [`FrameLook`], a size and a scale, so a
//! golden holds. Analytic coverage (a signed distance per pixel), no sampling, no text.

use casement_core::geometry::{LogicalSize, buffer_size};
use casement_core::window::{FrameColour, FrameLook, FrameMetrics, Light};
use ds_core::geometry::scale::Scale;

/// A raster: ARGB8888 (little-endian, premultiplied), rows top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Raster {
    /// Width in pixels.
    pub(crate) w: u32,
    /// Height in pixels.
    pub(crate) h: u32,
    /// The bytes.
    pub(crate) argb: Vec<u8>,
}

/// A premultiplied pixel, 0 to 1.
pub(super) type Px = [f32; 4];

/// `colour` laid over `dst` where `cover` (0 to 1) says.
pub(super) fn over(dst: &mut Px, colour: FrameColour, cover: f32) {
    let alpha = f32::from(colour.a) / 255.0 * cover.clamp(0.0, 1.0);
    let keep = 1.0 - alpha;
    let channels = [colour.r, colour.g, colour.b];
    for (slot, channel) in dst.iter_mut().zip(channels) {
        *slot = f32::from(channel) / 255.0 * alpha + *slot * keep;
    }
    dst[3] = alpha + dst[3] * keep;
}

/// The pixel's bytes, B G R A.
pub(super) fn bytes(px: Px) -> [u8; 4] {
    let level = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    [level(px[2]), level(px[1]), level(px[0]), level(px[3])]
}

/// How much of a pixel a shape covers, from the distance of its centre to the shape's edge in
/// logical pixels (negative inside) and the pixels per logical pixel.
pub(super) fn cover(distance: f32, per_logical: f32) -> f32 {
    (0.5 - distance * per_logical).clamp(0.0, 1.0)
}

/// The distance from `p` to the segment `a` to `b`.
fn segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let along = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (p.0 - a.0 - along * dx).hypot(p.1 - a.1 - along * dy)
}

/// A stroke from one point to another, in logical pixels.
type Stroke = ((f32, f32), (f32, f32));

/// A mark's strokes, relative to the light's centre, in logical pixels.
fn strokes(light: Light) -> &'static [Stroke] {
    match light {
        Light::Close => &[((-3.0, -3.0), (3.0, 3.0)), ((-3.0, 3.0), (3.0, -3.0))],
        Light::Minimise => &[((-3.2, 0.0), (3.2, 0.0))],
        Light::Zoom => &[((-3.4, 0.0), (3.4, 0.0)), ((0.0, -3.4), (0.0, 3.4))],
    }
}

/// Half the width of a mark's stroke, in logical pixels.
const STROKE: f32 = 0.6;

/// The width of a light's hairline, in logical pixels.
const HAIRLINE: f32 = 0.5;

/// Where a light's centre is in the titlebar, in logical pixels (the geometry's rule).
fn centre(metrics: &FrameMetrics, light: Light) -> (f32, f32) {
    let index = Light::ALL.iter().position(|l| *l == light).unwrap_or(0) as f32;
    let step = (metrics.light + metrics.gap) as f32;
    let spare = (metrics.titlebar.saturating_sub(metrics.light) / 2) as f32;
    (
        metrics.inset as f32 + index * step + metrics.light as f32 / 2.0,
        spare + metrics.light as f32 / 2.0,
    )
}

/// The titlebar over `width` logical pixels at `scale`: its ground with the top corners rounded,
/// the rule along its foot, the lights.
pub(crate) fn titlebar(
    look: &FrameLook,
    metrics: &FrameMetrics,
    width: u32,
    scale: Scale,
) -> Raster {
    let size = LogicalSize {
        w: width,
        h: metrics.titlebar,
    };
    let physical = buffer_size(size, scale);
    let (w, h) = (physical.w.max(1), physical.h.max(1));
    let per = scale.numerator() as f32 / Scale::DENOMINATOR as f32;
    let radius = look.radius as f32;
    let full = width as f32;
    let argb = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .flat_map(|(x, y)| {
            let at = ((x as f32 + 0.5) / per, (y as f32 + 0.5) / per);
            let mut px: Px = [0.0; 4];
            let corner = match (at.0 < radius, at.0 > full - radius, at.1 < radius) {
                (true, _, true) => (at.0 - radius).hypot(at.1 - radius) - radius,
                (_, true, true) => (at.0 - (full - radius)).hypot(at.1 - radius) - radius,
                _ => -1.0,
            };
            over(&mut px, look.titlebar, cover(corner, per));
            if y + 1 == h {
                let inside = px[3];
                over(&mut px, look.rule, inside);
            }
            for light in Light::ALL {
                let (cx, cy) = centre(metrics, light);
                let draw = look.light(light);
                let d = (at.0 - cx).hypot(at.1 - cy) - metrics.light as f32 / 2.0;
                over(&mut px, draw.ring, cover(d, per));
                over(&mut px, draw.fill, cover(d + HAIRLINE, per));
                if let Some(ink) = draw.mark {
                    let from_centre = (at.0 - cx, at.1 - cy);
                    let near = strokes(light)
                        .iter()
                        .map(|(a, b)| segment(from_centre, *a, *b))
                        .fold(f32::MAX, f32::min);
                    over(&mut px, ink, cover(near - STROKE, per));
                }
            }
            bytes(px)
        })
        .collect();
    Raster { w, h, argb }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::window::{LightDraw, ShadowLayer};

    const BLACK: FrameColour = FrameColour {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    const WHITE: FrameColour = FrameColour {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    fn look(mark: Option<FrameColour>) -> FrameLook {
        let light = LightDraw {
            fill: WHITE,
            ring: BLACK,
            mark,
        };
        FrameLook {
            radius: 10,
            titlebar: WHITE,
            rule: BLACK,
            lights: [light; 3],
            shadow: [ShadowLayer {
                dx: 0,
                dy: 10,
                blur: 20,
                spread: 0,
                colour: BLACK,
            }; 2],
        }
    }

    fn metrics() -> FrameMetrics {
        FrameMetrics::new(28, 12, 8, 13, 4, 12)
    }

    fn alpha_at(raster: &Raster, x: u32, y: u32) -> u8 {
        raster.argb[((y * raster.w + x) * 4 + 3) as usize]
    }

    #[test]
    fn the_titlebar_is_the_size_of_its_width_and_height_at_the_scale() {
        let one = titlebar(&look(None), &metrics(), 100, Scale::ONE);
        assert_eq!((one.w, one.h), (100, 28));
        let two = titlebar(&look(None), &metrics(), 100, Scale(240));
        assert_eq!((two.w, two.h), (200, 56));
        assert_eq!(two.argb.len(), 200 * 56 * 4);
    }

    #[test]
    fn the_top_corners_are_cut_and_the_rest_is_opaque() {
        let bar = titlebar(&look(None), &metrics(), 100, Scale::ONE);
        assert_eq!(
            alpha_at(&bar, 0, 0),
            0,
            "the corner pixel is outside the arc"
        );
        assert_eq!(alpha_at(&bar, 99, 0), 0);
        assert_eq!(alpha_at(&bar, 50, 0), 255);
        assert_eq!(alpha_at(&bar, 0, 27), 255, "the foot is square");
        assert_eq!(alpha_at(&bar, 99, 27), 255);
    }

    #[test]
    fn a_light_is_a_disc_and_its_mark_shows_only_when_asked() {
        let ink = FrameColour {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        let plain = titlebar(&look(None), &metrics(), 100, Scale::ONE);
        let marked = titlebar(&look(Some(ink)), &metrics(), 100, Scale::ONE);
        // The close light is centred at (19, 14): the middle is its fill, the mark's cross
        // crosses there, and a pixel in the corner of its square is the titlebar's ground.
        let at = |r: &Raster, x: u32, y: u32| r.argb[((y * r.w + x) * 4) as usize..][..4].to_vec();
        assert_eq!(at(&plain, 19, 14), vec![255, 255, 255, 255]);
        assert_ne!(at(&marked, 19, 14), at(&plain, 19, 14));
        assert_eq!(at(&marked, 13, 8), at(&plain, 13, 8));
    }
}
