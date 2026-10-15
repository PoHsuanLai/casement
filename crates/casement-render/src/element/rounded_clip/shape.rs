//! The shape a window's surfaces are clipped to: its content rectangle with the two bottom
//! corners rounded (the top meets the titlebar, which rounds its own corners). Pure numbers, so
//! the coverage the GLES shader computes (`gles.rs`, same formula) has a table test.

use crate::scene::OutputView;
use crate::scene::convert::{on_output, sub_of};
use casement_core::geometry::{LogicalPoint, LogicalRect, PhysicalRect};
use ds_core::geometry::scale::Scale;
use smithay::utils::{Physical, Rectangle};

/// A window's content rectangle on an output, in physical pixels, with its bottom corners
/// rounded by `radius` physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipShape {
    /// The content rectangle, relative to the output's top-left.
    rect: PhysicalRect,
    /// The corner radius in physical pixels (never more than half the width or the height).
    radius: u32,
}

impl ClipShape {
    /// The shape of `rect` (output pixels) with corners of `logical_radius` logical pixels at
    /// `scale`.
    ///
    /// ```
    /// use casement_core::geometry::PhysicalRect;
    /// use casement_render::ClipShape;
    /// use ds_core::geometry::scale::Scale;
    ///
    /// let rect = PhysicalRect { x: 0, y: 0, w: 200, h: 100 };
    /// let shape = ClipShape::new(rect, 10, Scale(180));
    /// assert_eq!(shape.radius(), 15);
    /// assert_eq!(shape.coverage(100.0, 50.0), 1.0);
    /// assert_eq!(shape.coverage(199.5, 99.5), 0.0);
    /// ```
    pub fn new(rect: PhysicalRect, logical_radius: u32, scale: Scale) -> ClipShape {
        let wanted = (f64::from(logical_radius) * scale.as_f64()).round() as u32;
        let radius = wanted.min(rect.w / 2).min(rect.h);
        ClipShape { rect, radius }
    }

    /// The shape of the window whose content is at `content` (global logical pixels) on `view`.
    pub fn of_content(content: LogicalRect, logical_radius: u32, view: &OutputView) -> ClipShape {
        let start = on_output(sub_of(content.origin), view);
        let end = on_output(
            sub_of(LogicalPoint {
                x: content
                    .origin
                    .x
                    .saturating_add(i32::try_from(content.size.w).unwrap_or(i32::MAX)),
                y: content
                    .origin
                    .y
                    .saturating_add(i32::try_from(content.size.h).unwrap_or(i32::MAX)),
            }),
            view,
        );
        let side = |n: i32| u32::try_from(n).unwrap_or(0);
        let rect = PhysicalRect {
            x: start.x,
            y: start.y,
            w: side(end.x - start.x),
            h: side(end.y - start.y),
        };
        ClipShape::new(rect, logical_radius, view.scale)
    }

    /// The corner radius in physical pixels.
    pub fn radius(&self) -> u32 {
        self.radius
    }

    /// The content rectangle in output pixels.
    pub fn rect(&self) -> PhysicalRect {
        self.rect
    }

    /// How much of the pixel centred at (`px`, `py`), in output pixels, the shape covers: 1
    /// inside, 0 outside, a ramp about one pixel wide across the edge. Only the bottom corners
    /// are round; the other two are square.
    pub fn coverage(&self, px: f32, py: f32) -> f32 {
        let (x, y) = (self.rect.x as f32, self.rect.y as f32);
        let (w, h) = (self.rect.w as f32, self.rect.h as f32);
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let r = match py > cy {
            true => self.radius as f32,
            false => 0.0,
        };
        let qx = (px - cx).abs() - w / 2.0 + r;
        let qy = (py - cy).abs() - h / 2.0 + r;
        let distance = qx.max(qy).min(0.0) + qx.max(0.0).hypot(qy.max(0.0)) - r;
        (0.5 - distance).clamp(0.0, 1.0)
    }

    /// Whether the rows `rows` (top, bottom; output pixels) reach the bottom corners' band, the
    /// only rows the clip changes. A pixel of slack either side keeps the anti-aliased edge.
    pub fn touches(&self, rows: (f64, f64)) -> bool {
        let bottom = f64::from(self.rect.y) + f64::from(self.rect.h);
        let band = f64::from(self.radius) + 1.0;
        self.radius > 0 && rows.1 > bottom - band && rows.0 < bottom + 1.0
    }

    /// The two squares of `radius` sides at the bottom corners, the only places the shape is
    /// not opaque over an opaque surface (so they leave the opaque regions).
    pub fn corner_squares(&self) -> [Rectangle<i32, Physical>; 2] {
        let r = i32::try_from(self.radius).unwrap_or(0);
        let w = i32::try_from(self.rect.w).unwrap_or(0);
        let h = i32::try_from(self.rect.h).unwrap_or(0);
        let top = self.rect.y + h - r;
        [
            Rectangle::new((self.rect.x, top).into(), (r, r).into()),
            Rectangle::new((self.rect.x + w - r, top).into(), (r, r).into()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: PhysicalRect = PhysicalRect {
        x: 0,
        y: 0,
        w: 200,
        h: 100,
    };

    /// Window radius 10 at three scales: the radius in pixels.
    #[test]
    fn the_radius_follows_the_scale() {
        for (scale, want) in [(120, 10), (180, 15), (240, 20)] {
            assert_eq!(ClipShape::new(RECT, 10, Scale(scale)).radius(), want);
        }
    }

    #[test]
    fn a_small_window_never_rounds_past_its_half() {
        let tiny = PhysicalRect {
            x: 0,
            y: 0,
            w: 8,
            h: 3,
        };
        assert_eq!(ClipShape::new(tiny, 10, Scale(120)).radius(), 3);
    }

    #[test]
    fn coverage_is_one_inside_zero_outside_and_a_half_on_the_curve() {
        let on_curve = |r: f32| {
            let along = r / std::f32::consts::SQRT_2;
            (200.0 - r + along, 100.0 - r + along)
        };
        for scale in [120_u32, 180, 240] {
            let shape = ClipShape::new(RECT, 10, Scale(scale));
            let r = shape.radius() as f32;
            let (curve_x, curve_y) = on_curve(r);
            let left_x = 200.0 - curve_x;
            // name, point, lowest, highest
            let table: [(&str, (f32, f32), f32, f32); 9] = [
                ("middle", (100.0, 50.0), 1.0, 1.0),
                ("bottom edge, mid", (100.0, 99.5), 1.0, 1.0),
                ("left edge, mid", (0.5, 50.0), 1.0, 1.0),
                ("top left stays square", (0.5, 0.5), 1.0, 1.0),
                ("top right stays square", (199.5, 0.5), 1.0, 1.0),
                ("bottom right corner pixel", (199.5, 99.5), 0.0, 0.0),
                ("bottom left corner pixel", (0.5, 99.5), 0.0, 0.0),
                ("on the right curve", (curve_x, curve_y), 0.49, 0.51),
                ("on the left curve", (left_x, curve_y), 0.49, 0.51),
            ];
            for (name, (x, y), low, high) in table {
                let got = shape.coverage(x, y);
                assert!(
                    (low..=high).contains(&got),
                    "{name} at scale {scale}: {got} not in {low}..={high}"
                );
            }
            assert_eq!(shape.coverage(-5.0, 50.0), 0.0, "left of the window");
            assert_eq!(shape.coverage(100.0, 130.0), 0.0, "below the window");
        }
    }

    #[test]
    fn the_curve_is_a_ramp_about_one_pixel_wide() {
        let shape = ClipShape::new(RECT, 10, Scale(120));
        // Along the diagonal of the right corner, from 0.7 px inside the curve to 0.7 px out.
        let step = std::f32::consts::FRAC_1_SQRT_2;
        let at = |distance: f32| {
            let along = 10.0 / std::f32::consts::SQRT_2 + distance * step;
            shape.coverage(190.0 + along, 90.0 + along)
        };
        assert_eq!(at(-0.7), 1.0);
        assert!((at(0.0) - 0.5).abs() < 0.01);
        assert_eq!(at(0.7), 0.0);
        assert!(at(-0.2) > at(0.2));
    }

    #[test]
    fn only_rows_in_the_corner_band_are_touched() {
        let shape = ClipShape::new(RECT, 10, Scale(120));
        // name, rows (top, bottom), touched; the band is 89 to 101 (radius 10 and a pixel)
        let cases = [
            ("the top", (0.0, 20.0), false),
            ("the middle", (40.0, 80.0), false),
            ("just above the band", (70.0, 89.0), false),
            ("the last rows", (90.0, 100.0), true),
            ("across the band edge", (80.0, 95.0), true),
            ("below", (101.0, 120.0), false),
        ];
        for (name, rows, want) in cases {
            assert_eq!(shape.touches(rows), want, "{name}");
        }
        let square = ClipShape::new(RECT, 0, Scale(120));
        assert!(!square.touches((90.0, 100.0)));
    }

    #[test]
    fn the_corner_squares_sit_at_the_bottom_corners() {
        let shape = ClipShape::new(
            PhysicalRect {
                x: 30,
                y: 40,
                w: 200,
                h: 100,
            },
            10,
            Scale(120),
        );
        let [left, right] = shape.corner_squares();
        assert_eq!((left.loc.x, left.loc.y, left.size.w), (30, 130, 10));
        assert_eq!((right.loc.x, right.loc.y, right.size.h), (220, 130, 10));
    }
}
