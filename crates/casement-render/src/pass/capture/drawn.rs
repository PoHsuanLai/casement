//! A client's cursor as the screen draws it (wave-2026-10-15/capgpu): the buffer is turned
//! upright by its `wl_surface.set_buffer_transform`, cut to the viewport's source rectangle
//! (`wp_viewport.set_source`) and only then scaled to the size the cursor is shown at.
//!
//! The rule is smithay's own: a surface-logical point `p` of the upright surface sits in the
//! buffer at `transform_point_in(p)` times the buffer scale, so the upright image is the buffer
//! turned by the transform's inverse. The scaling to the drawn size (the viewport's
//! destination, then the output's scale) and the output's own turn stay in
//! [`ArgbImage::fitted`].

use super::ArgbImage;
use super::bitmap::sides;
use casement_core::geometry::{PhysicalRect, PhysicalSize, Transform};

/// The viewport's source rectangle, in surface-logical units (buffer pixels divided by the
/// buffer scale, after the buffer transform), as smithay's `SurfaceView::src` has it.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct SourceRect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width.
    pub w: f64,
    /// Height.
    pub h: f64,
}

impl SourceRect {
    /// A rectangle.
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> SourceRect {
        SourceRect { x, y, w, h }
    }
}

/// How a surface's buffer is turned into the picture the screen shows.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct DrawnView {
    /// The surface's buffer scale (1 when the client sets none).
    pub buffer_scale: i32,
    /// The surface's buffer transform.
    pub buffer_transform: Transform,
    /// The viewport's source rectangle.
    pub src: SourceRect,
}

impl DrawnView {
    /// A view.
    pub fn new(buffer_scale: i32, buffer_transform: Transform, src: SourceRect) -> DrawnView {
        DrawnView {
            buffer_scale,
            buffer_transform,
            src,
        }
    }
}

/// The transform that undoes `transform`: the quarter turns swap, every other one is its own.
fn inverse(transform: Transform) -> Transform {
    match transform {
        Transform::R90 => Transform::R270,
        Transform::R270 => Transform::R90,
        Transform::Flipped90 => Transform::Flipped270,
        Transform::Flipped270 => Transform::Flipped90,
        other => other,
    }
}

/// The part of the upright image of `upright` pixels that `view`'s source rectangle covers:
/// each edge is rounded to the nearest pixel and kept inside the image, and the part is at
/// least one pixel on a side while the image has any.
///
/// ```
/// use casement_core::geometry::{PhysicalSize, Transform};
/// use casement_render::pass::{DrawnView, SourceRect, crop_pixels};
///
/// let view = DrawnView::new(2, Transform::Normal, SourceRect::new(1.0, 0.0, 2.0, 4.0));
/// let part = crop_pixels(PhysicalSize { w: 8, h: 8 }, &view);
/// assert_eq!((part.x, part.y, part.w, part.h), (2, 0, 4, 8));
/// ```
pub fn crop_pixels(upright: PhysicalSize, view: &DrawnView) -> PhysicalRect {
    let scale = f64::from(view.buffer_scale.max(1));
    let edge = |near: f64, extent: f64, limit: u32| -> (u32, u32) {
        let to_px = |v: f64| {
            u32::try_from((v * scale).round().clamp(0.0, f64::from(limit)) as i64).unwrap_or(0)
        };
        let (a, b) = (to_px(near), to_px(near + extent));
        let a = a.min(limit.saturating_sub(1));
        (a, b.max(a.saturating_add(1)).min(limit))
    };
    let (x0, x1) = edge(view.src.x, view.src.w, upright.w);
    let (y0, y1) = edge(view.src.y, view.src.h, upright.h);
    PhysicalRect {
        x: i32::try_from(x0).unwrap_or(0),
        y: i32::try_from(y0).unwrap_or(0),
        w: x1.saturating_sub(x0),
        h: y1.saturating_sub(y0),
    }
}

impl ArgbImage {
    /// This image, a surface's buffer, as the screen shows it before scaling: turned upright by
    /// the buffer transform and cut to the viewport's source rectangle. The result still has the
    /// buffer's pixel density; [`ArgbImage::fitted`] scales it to the drawn size.
    ///
    /// ```
    /// use casement_core::geometry::{PhysicalSize, Transform};
    /// use casement_render::pass::{ArgbImage, DrawnView, SourceRect};
    ///
    /// let buffer = ArgbImage::new(PhysicalSize { w: 2, h: 1 }, vec![1, 1, 1, 255, 2, 2, 2, 255]);
    /// let view = DrawnView::new(1, Transform::Normal, SourceRect::new(1.0, 0.0, 1.0, 1.0));
    /// assert_eq!(buffer.drawn(&view).argb, [2, 2, 2, 255]);
    /// ```
    pub fn drawn(&self, view: &DrawnView) -> ArgbImage {
        let upright = self.turned(inverse(view.buffer_transform));
        upright.cut(crop_pixels(upright.size, view))
    }

    /// The part inside `rect`, which lies inside the image.
    fn cut(&self, rect: PhysicalRect) -> ArgbImage {
        let (w, h) = sides(self.size);
        let (x, y) = (
            usize::try_from(rect.x).unwrap_or(0),
            usize::try_from(rect.y).unwrap_or(0),
        );
        let (cw, ch) = sides(PhysicalSize {
            w: rect.w,
            h: rect.h,
        });
        let mut argb = Vec::with_capacity(cw.saturating_mul(ch).saturating_mul(4));
        for line in y..y.saturating_add(ch) {
            let start = line.saturating_mul(w).saturating_add(x).saturating_mul(4);
            let end = start.saturating_add(cw.saturating_mul(4));
            match (line < h, self.argb.get(start..end)) {
                (true, Some(row)) => argb.extend_from_slice(row),
                _ => argb.resize(argb.len().saturating_add(cw.saturating_mul(4)), 0),
            }
        }
        ArgbImage {
            size: PhysicalSize {
                w: rect.w,
                h: rect.h,
            },
            argb,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(w: u32, h: u32) -> PhysicalSize {
        PhysicalSize { w, h }
    }

    /// An image whose pixel `n` is `[n, n, n, 255]`.
    fn counted(w: u32, h: u32) -> ArgbImage {
        let argb = (0..w * h)
            .flat_map(|n| {
                let n = u8::try_from(n).unwrap_or(0);
                [n, n, n, 255]
            })
            .collect();
        ArgbImage::new(size(w, h), argb)
    }

    fn ids(image: &ArgbImage) -> Vec<u8> {
        image.argb.chunks(4).map(|pixel| pixel[0]).collect()
    }

    fn whole(w: f64, h: f64) -> SourceRect {
        SourceRect::new(0.0, 0.0, w, h)
    }

    #[test]
    fn the_crop_follows_the_source_rectangle_and_the_buffer_scale() {
        // name, upright size, scale, source (x, y, w, h), expected (x, y, w, h)
        type Case = (
            &'static str,
            (u32, u32),
            i32,
            (f64, f64, f64, f64),
            (i32, i32, u32, u32),
        );
        let table: Vec<Case> = vec![
            ("whole", (8, 8), 1, (0.0, 0.0, 8.0, 8.0), (0, 0, 8, 8)),
            ("a corner", (8, 8), 1, (2.0, 3.0, 4.0, 2.0), (2, 3, 4, 2)),
            ("doubled", (8, 8), 2, (1.0, 0.0, 2.0, 4.0), (2, 0, 4, 8)),
            (
                "fractional edges round",
                (8, 8),
                1,
                (0.4, 0.6, 2.2, 2.2),
                (0, 1, 2, 2),
            ),
            (
                "past the image is cut",
                (8, 8),
                1,
                (6.0, 6.0, 8.0, 8.0),
                (6, 6, 2, 2),
            ),
            (
                "a sliver keeps a pixel",
                (8, 8),
                1,
                (3.0, 3.0, 0.1, 0.1),
                (3, 3, 1, 1),
            ),
            (
                "outside keeps the last pixel",
                (8, 8),
                1,
                (20.0, 20.0, 2.0, 2.0),
                (7, 7, 1, 1),
            ),
        ];
        for (name, (w, h), scale, (x, y, sw, sh), (ex, ey, ew, eh)) in table {
            let view = DrawnView::new(scale, Transform::Normal, SourceRect::new(x, y, sw, sh));
            let part = crop_pixels(size(w, h), &view);
            assert_eq!((part.x, part.y, part.w, part.h), (ex, ey, ew, eh), "{name}");
        }
    }

    #[test]
    fn a_buffer_transform_is_undone_before_the_cut() {
        // A B C / D E F as 0 1 2 / 3 4 5, held in a buffer turned by the transform: the upright
        // surface is the buffer turned back, so turning the surface forward gives the buffer.
        for transform in [
            Transform::Normal,
            Transform::R90,
            Transform::R180,
            Transform::R270,
            Transform::Flipped,
            Transform::Flipped90,
            Transform::Flipped180,
            Transform::Flipped270,
        ] {
            let surface = counted(3, 2);
            let buffer = surface.fitted(size(3, 2), transform);
            let back = buffer.drawn(&DrawnView::new(1, transform, whole(3.0, 2.0)));
            assert_eq!(back.size, size(3, 2), "{transform:?}");
            assert_eq!(ids(&back), ids(&surface), "{transform:?}");
        }
    }

    #[test]
    fn a_viewport_crop_then_a_turn_keeps_the_right_pixels() {
        // 4 x 2 upright surface 0..7; the viewport takes the right two columns.
        let surface = counted(4, 2);
        let buffer = surface.fitted(size(4, 2), Transform::R90);
        assert_eq!(buffer.size, size(2, 4));
        let view = DrawnView::new(1, Transform::R90, SourceRect::new(2.0, 0.0, 2.0, 2.0));
        let drawn = buffer.drawn(&view);
        assert_eq!(drawn.size, size(2, 2));
        assert_eq!(ids(&drawn), [2, 3, 6, 7]);
    }
}
