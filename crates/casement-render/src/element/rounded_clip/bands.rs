//! The rounded clip for every draw that has no corner shader: Pixman, the dummy renderer, a
//! multi-GPU copy, and on GLES a surface the shader does not cover (a turned or flipped texture,
//! a single-pixel buffer).
//!
//! Nothing here reads the texture. The window is drawn in pieces: the body (everything outside
//! the two corner squares) and the fully covered pixels of the squares at full strength, then
//! each partly covered pixel of the curve alone, with the surface's alpha scaled by its
//! coverage. The pieces are `damage` rectangles in the destination's space, so a texture's own
//! transform (turned, flipped, y-inverted) never enters: the renderer applies it to the whole
//! surface, we only choose where and how strongly it draws. Coverage is `ClipShape::coverage`
//! at the pixel's centre, in `LEVELS` steps (a pixel is drawn at the nearest step), so a corner
//! costs at most `LEVELS` extra draws however large the radius.

use super::{ClipShape, RoundedClipElement};
use crate::compositor::Compositor;
use smithay::backend::renderer::Frame;
use smithay::backend::renderer::element::Element;
use smithay::backend::renderer::element::surface::WaylandSurfaceTexture;
use smithay::utils::{Buffer as BufferCoords, Physical, Rectangle};
use std::collections::BTreeMap;

/// How many steps of coverage a pixel of the curve is drawn at (a step is `1/LEVELS` of the
/// surface's alpha; the error is under `1/(2 * LEVELS)`).
pub(super) const LEVELS: u8 = 32;

type Rect = Rectangle<i32, Physical>;

/// The pieces of a clipped draw, in the destination's space.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Plan {
    /// Drawn at full strength.
    pub(super) solid: Vec<Rect>,
    /// Single pixels drawn at `level / LEVELS` of the surface's alpha, by level (1 to `LEVELS - 1`).
    pub(super) partial: BTreeMap<u8, Vec<Rect>>,
}

/// The step `coverage` (0 to 1) is drawn at.
fn level_of(coverage: f32) -> u8 {
    (coverage * f32::from(LEVELS))
        .round()
        .clamp(0.0, f32::from(LEVELS)) as u8
}

/// The pixels of `shape`'s corner squares (output pixels) by how they draw: full runs along
/// a row, and single partial pixels by level.
fn corner_pieces(shape: &ClipShape) -> (Vec<Rect>, BTreeMap<u8, Vec<Rect>>) {
    let mut runs = Vec::new();
    let mut partial: BTreeMap<u8, Vec<Rect>> = BTreeMap::new();
    for square in shape.corner_squares() {
        let (left, top, radius) = (square.loc.x, square.loc.y, square.size.w);
        for y in top..top + radius {
            let mut start: Option<i32> = None;
            for x in left..=left + radius {
                let level = match x < left + radius {
                    true => level_of(shape.coverage(x as f32 + 0.5, y as f32 + 0.5)),
                    false => 0,
                };
                match (level == LEVELS, start) {
                    (true, None) => start = Some(x),
                    (false, Some(from)) => {
                        runs.push(Rectangle::new((from, y).into(), (x - from, 1).into()));
                        start = None;
                    }
                    _ => {}
                }
                if level > 0 && level < LEVELS {
                    partial
                        .entry(level)
                        .or_default()
                        .push(Rectangle::new((x, y).into(), (1, 1).into()));
                }
            }
        }
    }
    (runs, partial)
}

/// `rects` limited to `damage`, both in the same space.
fn within(rects: &[Rect], damage: &[Rect]) -> Vec<Rect> {
    rects
        .iter()
        .flat_map(|rect| damage.iter().filter_map(move |d| d.intersection(*rect)))
        .collect()
}

/// `rects` (output pixels) moved to be relative to `dst`.
fn relative(rects: &[Rect], dst: Rect) -> Vec<Rect> {
    rects
        .iter()
        .map(|rect| Rectangle::new(rect.loc - dst.loc, rect.size))
        .collect()
}

/// How `shape` draws a surface at `dst` where `damage` (relative to `dst`) asks.
pub(super) fn plan(shape: &ClipShape, dst: Rect, damage: &[Rect]) -> Plan {
    let squares = relative(&shape.corner_squares(), dst);
    let (runs, partial) = corner_pieces(shape);
    let body: Vec<Rect> = damage
        .iter()
        .flat_map(|d| d.subtract_rects(squares.iter().copied()))
        .collect();
    let solid = body
        .into_iter()
        .chain(within(&relative(&runs, dst), damage))
        .collect();
    let partial = partial
        .into_iter()
        .map(|(level, pixels)| (level, within(&relative(&pixels, dst), damage)))
        .filter(|(_, pixels)| !pixels.is_empty())
        .collect();
    Plan { solid, partial }
}

/// Draws `clip`'s surface at `dst` where `damage` (relative to `dst`) asks, clipped to its
/// shape, on any renderer.
pub(crate) fn draw<R: Compositor>(
    frame: &mut R::Frame<'_, '_>,
    clip: &RoundedClipElement<R>,
    src: Rectangle<f64, BufferCoords>,
    dst: Rect,
    damage: &[Rect],
    opaque_regions: &[Rect],
) -> Result<(), R::Error> {
    let pieces = plan(&clip.shape(), dst, damage);
    paint(
        frame,
        clip,
        (src, dst),
        (pieces.solid.as_slice(), opaque_regions),
        1.0,
    )?;
    pieces.partial.iter().try_for_each(|(level, pixels)| {
        let weight = f32::from(*level) / f32::from(LEVELS);
        paint(
            frame,
            clip,
            (src, dst),
            (pixels.as_slice(), &[][..]),
            weight,
        )
    })
}

/// The surface drawn where `damage` asks with its alpha times `weight`.
fn paint<R: Compositor>(
    frame: &mut R::Frame<'_, '_>,
    clip: &RoundedClipElement<R>,
    (src, dst): (Rectangle<f64, BufferCoords>, Rect),
    (damage, opaque_regions): (&[Rect], &[Rect]),
    weight: f32,
) -> Result<(), R::Error> {
    if damage.is_empty() {
        return Ok(());
    }
    let surface = clip.surface();
    let alpha = surface.alpha() * weight;
    match surface.texture() {
        WaylandSurfaceTexture::Texture(texture) => frame.render_texture_from_to(
            texture,
            src,
            dst,
            damage,
            opaque_regions,
            surface.transform(),
            alpha,
        ),
        WaylandSurfaceTexture::SolidColor(color) => frame.draw_solid(dst, damage, *color * alpha),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::PhysicalRect;
    use ds_core::geometry::scale::Scale;

    fn shape(x: i32, y: i32, scale: u32) -> ClipShape {
        let rect = PhysicalRect {
            x,
            y,
            w: 120,
            h: 80,
        };
        ClipShape::new(rect, 10, Scale(scale))
    }

    fn area(rects: &[Rect]) -> i64 {
        rects
            .iter()
            .map(|r| i64::from(r.size.w) * i64::from(r.size.h))
            .sum()
    }

    /// How strongly the plan draws the pixel at (x, y), relative to dst.
    fn strength(plan: &Plan, x: i32, y: i32) -> f32 {
        let at = Rectangle::<i32, Physical>::new((x, y).into(), (1, 1).into());
        let hit = |rects: &[Rect]| rects.iter().filter(|r| r.overlaps(at)).count();
        let partial: Vec<(u8, usize)> = plan
            .partial
            .iter()
            .map(|(level, rects)| (*level, hit(rects)))
            .collect();
        assert!(
            hit(&plan.solid) + partial.iter().map(|(_, n)| n).sum::<usize>() <= 1,
            "({x}, {y}) is drawn twice"
        );
        match (hit(&plan.solid), partial.iter().find(|(_, n)| *n == 1)) {
            (1, _) => 1.0,
            (_, Some((level, _))) => f32::from(*level) / f32::from(LEVELS),
            _ => 0.0,
        }
    }

    /// Every pixel of the window is drawn at the shape's coverage, to the nearest step, once
    /// at most; at three scales and two places.
    #[test]
    fn every_pixel_draws_at_the_shapes_coverage() {
        for (scale, x, y) in [(120, 0, 0), (180, 0, 0), (240, 0, 0), (180, 33, 17)] {
            let clip = shape(x, y, scale);
            let dst = Rectangle::new((x, y).into(), (120, 80).into());
            let whole = Rectangle::from_size((120, 80).into());
            let made = plan(&clip, dst, &[whole]);
            for py in 0..80 {
                for px in 0..120 {
                    let want = clip.coverage((x + px) as f32 + 0.5, (y + py) as f32 + 0.5);
                    let got = strength(&made, px, py);
                    let limit = 0.5 / f32::from(LEVELS) + 1e-4;
                    assert!(
                        (got - want).abs() <= limit,
                        "scale {scale} pixel ({px}, {py}): drew {got}, covers {want}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_corner_pixels_are_not_drawn_and_the_top_ones_are() {
        let clip = shape(0, 0, 120);
        let dst = Rectangle::new((0, 0).into(), (120, 80).into());
        let made = plan(&clip, dst, &[Rectangle::from_size((120, 80).into())]);
        assert_eq!(strength(&made, 0, 79), 0.0);
        assert_eq!(strength(&made, 119, 79), 0.0);
        assert_eq!(strength(&made, 0, 0), 1.0);
        assert_eq!(strength(&made, 119, 0), 1.0);
        assert_eq!(strength(&made, 60, 79), 1.0);
    }

    #[test]
    fn only_the_damaged_part_is_drawn() {
        let clip = shape(0, 0, 120);
        let dst = Rectangle::new((0, 0).into(), (120, 80).into());
        let damage = [Rectangle::<i32, Physical>::new(
            (0, 70).into(),
            (8, 10).into(),
        )];
        let made = plan(&clip, dst, &damage);
        let drawn = area(&made.solid) + made.partial.values().map(|p| area(p)).sum::<i64>();
        assert!(drawn <= 80 && drawn > 0, "{drawn}");
        assert_eq!(strength(&made, 60, 40), 0.0);
        assert_eq!(strength(&made, 5, 75), clip.coverage(5.5, 75.5).round());
        // An empty damage draws nothing.
        assert_eq!(plan(&clip, dst, &[]), Plan::default());
    }

    #[test]
    fn a_corner_costs_at_most_one_draw_per_step() {
        let clip = shape(0, 0, 240);
        let dst = Rectangle::new((0, 0).into(), (120, 80).into());
        let made = plan(&clip, dst, &[Rectangle::from_size((120, 80).into())]);
        assert!(made.partial.len() < usize::from(LEVELS));
        assert!(made.partial.keys().all(|l| (1..LEVELS).contains(l)));
    }

    #[test]
    fn a_surface_inside_the_window_gets_the_part_of_the_curve_it_covers() {
        // A subsurface in the bottom left of the window: its dst starts inside the square.
        let clip = shape(100, 50, 120);
        let dst = Rectangle::new((104, 120).into(), (30, 10).into());
        let made = plan(&clip, dst, &[Rectangle::from_size((30, 10).into())]);
        // Window pixel (104, 129) is 4 px in on the bottom row, which the curve cuts.
        assert!(strength(&made, 0, 9) < 1.0);
        assert_eq!(strength(&made, 29, 0), 1.0);
    }
}
