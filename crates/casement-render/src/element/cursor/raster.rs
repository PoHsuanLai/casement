//! The compositor's own cursors, rasterised at an output's native scale (A18): black shapes
//! with a white outline in the Mac manner, drawn from a few primitives by signed distance,
//! sixteen samples a pixel. Pure integer output for a given scale, so a golden holds.
//!
//! The shapes are casement's own, drawn here: no cursor theme is read (themes arrive with the
//! settings that name one; FINDINGS).

use crate::scene::CursorShape;
use casement_core::geometry::{LogicalPoint, LogicalSize, buffer_size};
use ds_core::geometry::scale::Scale;

/// A cursor's canvas side in logical pixels.
pub(crate) const CANVAS: u32 = 24;

/// The outline's width in logical pixels.
const OUTLINE: f64 = 1.0;

/// Samples per pixel side (16 a pixel).
const SAMPLES: u32 = 4;

/// One cursor raster: a square ARGB8888 buffer (little-endian, premultiplied).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Raster {
    /// Its side in physical pixels.
    pub(crate) side: u32,
    /// Its pixels, rows top to bottom.
    pub(crate) argb: Vec<u8>,
}

/// Where the hotspot sits on the canvas.
pub(crate) fn hotspot(shape: CursorShape) -> LogicalPoint {
    let (x, y) = match shape {
        CursorShape::Default => (5, 3),
        CursorShape::Pointer => (9, 3),
        _ => (12, 12),
    };
    LogicalPoint { x, y }
}

/// `shape` at `scale`.
pub(crate) fn raster(shape: CursorShape, scale: Scale) -> Raster {
    let canvas = LogicalSize {
        w: CANVAS,
        h: CANVAS,
    };
    let side = buffer_size(canvas, scale).w.max(1);
    let prims = primitives(shape);
    let step = f64::from(CANVAS) / f64::from(side);
    let argb = (0..side)
        .flat_map(|y| (0..side).map(move |x| (x, y)))
        .flat_map(|(x, y)| pixel(&prims, x, y, step))
        .collect();
    Raster { side, argb }
}

/// One pixel's bytes (B, G, R, A): white where only the outline covers, black where the body
/// does.
fn pixel(prims: &[Prim], x: u32, y: u32, step: f64) -> [u8; 4] {
    let (body, outline) = (0..SAMPLES * SAMPLES)
        .map(|n| {
            let at =
                |p: u32, i: u32| (f64::from(p) + (f64::from(i) + 0.5) / f64::from(SAMPLES)) * step;
            distance(prims, (at(x, n % SAMPLES), at(y, n / SAMPLES)))
        })
        .fold((0_u32, 0_u32), |(body, outline), d| {
            (
                body + u32::from(d <= 0.0),
                outline + u32::from(d <= OUTLINE),
            )
        });
    let level = |count: u32| {
        let samples = SAMPLES * SAMPLES;
        u8::try_from((count * 255 + samples / 2) / samples).unwrap_or(u8::MAX)
    };
    let white = level(outline - body);
    [white, white, white, level(outline)]
}

/// A point on the canvas.
type P = (f64, f64);

/// A primitive of a cursor's body.
#[derive(Debug, Clone, PartialEq)]
enum Prim {
    /// A closed polygon.
    Polygon(Vec<P>),
    /// A segment with round ends of this radius.
    Capsule(P, P, f64),
    /// A circle's band: centre, radius, width.
    Ring(P, f64, f64),
}

/// The signed distance from `p` to the union of `prims` (negative inside).
fn distance(prims: &[Prim], p: P) -> f64 {
    prims
        .iter()
        .map(|prim| match prim {
            Prim::Polygon(points) => polygon(points, p),
            Prim::Capsule(a, b, r) => segment(*a, *b, p) - r,
            Prim::Ring(c, r, w) => (length(sub(p, *c)) - r).abs() - w / 2.0,
        })
        .fold(f64::INFINITY, f64::min)
}

fn sub(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}

fn dot(a: P, b: P) -> f64 {
    a.0 * b.0 + a.1 * b.1
}

fn length(a: P) -> f64 {
    dot(a, a).sqrt()
}

/// The distance from `p` to the segment `a`-`b`.
fn segment(a: P, b: P, p: P) -> f64 {
    let ab = sub(b, a);
    let ap = sub(p, a);
    let t = (dot(ap, ab) / dot(ab, ab).max(f64::EPSILON)).clamp(0.0, 1.0);
    length(sub(ap, (ab.0 * t, ab.1 * t)))
}

/// The signed distance to a closed polygon (even-odd inside test).
fn polygon(points: &[P], p: P) -> f64 {
    let edges = points.iter().zip(points.iter().cycle().skip(1));
    let (nearest, inside) = edges.fold((f64::INFINITY, false), |(nearest, inside), (a, b)| {
        let crosses =
            (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0;
        (nearest.min(segment(*a, *b, p)), inside ^ crosses)
    });
    if inside { -nearest } else { nearest }
}

/// The body of `shape` on its canvas.
fn primitives(shape: CursorShape) -> Vec<Prim> {
    match shape {
        CursorShape::Default => vec![Prim::Polygon(vec![
            (5.0, 3.0),
            (16.5, 14.3),
            (11.4, 14.3),
            (14.0, 20.2),
            (11.6, 21.2),
            (9.0, 15.3),
            (5.0, 19.0),
        ])],
        CursorShape::Text => vec![
            Prim::Capsule((12.0, 5.0), (12.0, 19.0), 0.8),
            Prim::Capsule((9.5, 4.2), (14.5, 4.2), 0.8),
            Prim::Capsule((9.5, 19.8), (14.5, 19.8), 0.8),
        ],
        CursorShape::Pointer => {
            let mut hand = palm(1.0);
            hand.extend([
                Prim::Capsule((9.0, 4.7), (9.0, 12.0), 1.7),
                Prim::Capsule((12.6, 10.4), (12.6, 13.5), 1.6),
                Prim::Capsule((15.8, 11.0), (15.8, 14.0), 1.6),
                Prim::Capsule((18.8, 12.2), (18.8, 15.0), 1.5),
            ]);
            hand
        }
        CursorShape::Grab => {
            let mut hand = palm(0.0);
            hand.extend([
                Prim::Capsule((8.2, 6.4), (8.2, 12.0), 1.5),
                Prim::Capsule((11.2, 4.8), (11.2, 12.0), 1.5),
                Prim::Capsule((14.2, 5.0), (14.2, 12.0), 1.5),
                Prim::Capsule((17.0, 6.8), (17.0, 13.0), 1.4),
            ]);
            hand
        }
        CursorShape::Grabbing => {
            let mut hand = palm(0.0);
            hand.extend([
                Prim::Capsule((8.4, 9.6), (8.4, 11.5), 1.5),
                Prim::Capsule((11.4, 8.9), (11.4, 11.5), 1.5),
                Prim::Capsule((14.4, 9.1), (14.4, 11.5), 1.5),
                Prim::Capsule((17.2, 10.2), (17.2, 12.5), 1.4),
            ]);
            hand
        }
        CursorShape::ResizeEw => double_arrow(0.0),
        CursorShape::ResizeNs => double_arrow(90.0),
        CursorShape::ResizeNwse => double_arrow(45.0),
        CursorShape::ResizeNesw => double_arrow(-45.0),
        CursorShape::Wait => vec![
            Prim::Ring((12.0, 12.0), 6.5, 2.2),
            Prim::Capsule((12.0, 12.0), (12.0, 8.2), 1.0),
            Prim::Capsule((12.0, 12.0), (14.8, 12.0), 1.0),
        ],
        CursorShape::NotAllowed => vec![
            Prim::Ring((12.0, 12.0), 6.5, 2.2),
            Prim::Capsule((7.6, 7.6), (16.4, 16.4), 1.1),
        ],
    }
}

/// A palm and thumb, `lift` logical pixels lower for the pointing hand.
fn palm(lift: f64) -> Vec<Prim> {
    let down = |(x, y): P| (x, y + lift);
    vec![
        Prim::Polygon(
            [
                (6.6, 11.0),
                (18.6, 11.4),
                (19.8, 16.0),
                (16.8, 20.6),
                (9.4, 20.6),
                (6.4, 17.2),
            ]
            .into_iter()
            .map(down)
            .collect(),
        ),
        Prim::Capsule(down((4.6, 13.2)), down((7.4, 16.2)), 1.4),
    ]
}

/// A two-headed arrow through the canvas centre, turned `degrees` clockwise from horizontal.
fn double_arrow(degrees: f64) -> Vec<Prim> {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let turn = |(x, y): P| (12.0 + x * cos - y * sin, 12.0 + x * sin + y * cos);
    let head = |sign: f64| {
        Prim::Polygon(
            [(9.0 * sign, 0.0), (4.4 * sign, -4.2), (4.4 * sign, 4.2)]
                .into_iter()
                .map(turn)
                .collect(),
        )
    };
    vec![
        Prim::Capsule(turn((-5.0, 0.0)), turn((5.0, 0.0)), 1.0),
        head(1.0),
        head(-1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [CursorShape; 11] = [
        CursorShape::Default,
        CursorShape::Text,
        CursorShape::Pointer,
        CursorShape::Grab,
        CursorShape::Grabbing,
        CursorShape::ResizeNs,
        CursorShape::ResizeEw,
        CursorShape::ResizeNesw,
        CursorShape::ResizeNwse,
        CursorShape::Wait,
        CursorShape::NotAllowed,
    ];

    fn at(raster: &Raster, x: u32, y: u32) -> [u8; 4] {
        let i = usize::try_from((y * raster.side + x) * 4).expect("index fits");
        [
            raster.argb[i],
            raster.argb[i + 1],
            raster.argb[i + 2],
            raster.argb[i + 3],
        ]
    }

    #[test]
    fn rasters_are_native_size_and_premultiplied() {
        for (scale, side) in [(120, 24), (150, 30), (180, 36), (240, 48)] {
            for shape in ALL {
                let raster = raster(shape, Scale(scale));
                assert_eq!(raster.side, side, "{shape:?} at {scale}");
                assert_eq!(raster.argb.len(), (side * side * 4) as usize);
                assert!(
                    raster
                        .argb
                        .chunks(4)
                        .all(|px| px[0] <= px[3] && px[0] == px[1] && px[1] == px[2]),
                    "{shape:?} at {scale}: grey and premultiplied"
                );
            }
        }
    }

    #[test]
    fn every_shape_has_a_black_body_inside_a_white_outline() {
        for shape in ALL {
            let raster = raster(shape, Scale(240));
            let pixels: Vec<[u8; 4]> = raster
                .argb
                .chunks(4)
                .map(|px| [px[0], px[1], px[2], px[3]])
                .collect();
            assert!(
                pixels.contains(&[0, 0, 0, 255]),
                "{shape:?} has an opaque black body"
            );
            assert!(
                pixels.contains(&[255, 255, 255, 255]),
                "{shape:?} has an opaque white outline"
            );
            assert!(
                pixels.contains(&[0, 0, 0, 0]),
                "{shape:?} leaves the canvas corners clear"
            );
        }
    }

    #[test]
    fn the_arrow_tip_is_at_its_hotspot() {
        let raster = raster(CursorShape::Default, Scale(120));
        let tip = hotspot(CursorShape::Default);
        let (x, y) = (
            u32::try_from(tip.x).expect("x"),
            u32::try_from(tip.y).expect("y"),
        );
        assert_ne!(at(&raster, x, y + 1)[3], 0, "covered just below the tip");
        assert_eq!(at(&raster, x - 3, y - 2)[3], 0, "clear up and left of it");
    }

    #[test]
    fn rasterising_is_deterministic() {
        for shape in ALL {
            assert_eq!(raster(shape, Scale(180)), raster(shape, Scale(180)));
        }
    }
}
