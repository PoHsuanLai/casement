//! A Space's picture for the shell's Spaces bar: the windows of the Space, each drawn alone
//! (window capture's pixels), shrunk into a small image where they sit on the output. Plain
//! integer maths over premultiplied RGBA; the background stays transparent because the shell
//! draws the wallpaper under it.

use casement_core::geometry::{LogicalRect, PhysicalRect, PhysicalSize};

/// One window's pixels and where they go in the picture.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ThumbLayer<'a> {
    /// Where the window goes, in picture pixels (it may reach outside the picture).
    pub dest: PhysicalRect,
    /// The size of the window's own pixels.
    pub src: PhysicalSize,
    /// The window's pixels, premultiplied RGBA, rows top first.
    pub rgba: &'a [u8],
}

impl<'a> ThumbLayer<'a> {
    /// The window pixels `rgba` of size `src`, drawn into `dest`.
    pub fn new(dest: PhysicalRect, src: PhysicalSize, rgba: &'a [u8]) -> ThumbLayer<'a> {
        ThumbLayer { dest, src, rgba }
    }
}

/// Where a window of `window` (global logical pixels) lands in a picture of `picture` pixels
/// of the output `output` (global logical pixels): both edges are scaled and rounded, so
/// windows that touch still touch.
///
/// ```
/// use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, PhysicalSize};
/// use casement_render::pass::thumb_dest;
///
/// let rect = |x, y, w, h| LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h });
/// let dest = thumb_dest(rect(960, 540, 960, 540), rect(0, 0, 1920, 1080), PhysicalSize { w: 192, h: 108 });
/// assert_eq!((dest.x, dest.y, dest.w, dest.h), (96, 54, 96, 54));
/// ```
pub fn thumb_dest(window: LogicalRect, output: LogicalRect, picture: PhysicalSize) -> PhysicalRect {
    let edge = |from: i32, origin: i32, span: u32, pixels: u32| -> i32 {
        let offset = i64::from(from) - i64::from(origin);
        let span = i64::from(span.max(1));
        let scaled = (offset * i64::from(pixels) * 2 + span * offset.signum()) / (span * 2);
        i32::try_from(scaled).unwrap_or(0)
    };
    let left = edge(window.origin.x, output.origin.x, output.size.w, picture.w);
    let top = edge(window.origin.y, output.origin.y, output.size.h, picture.h);
    let right = edge(
        window.origin.x.saturating_add_unsigned(window.size.w),
        output.origin.x,
        output.size.w,
        picture.w,
    );
    let bottom = edge(
        window.origin.y.saturating_add_unsigned(window.size.h),
        output.origin.y,
        output.size.h,
        picture.h,
    );
    PhysicalRect {
        x: left,
        y: top,
        w: u32::try_from(right.saturating_sub(left)).unwrap_or(0),
        h: u32::try_from(bottom.saturating_sub(top)).unwrap_or(0),
    }
}

/// The picture of `size` with `layers` drawn into it, back first: premultiplied RGBA, rows top
/// first, transparent where no window is. Each picture pixel averages the box of window pixels
/// it covers (at least one), so a shrunk window is smooth, not jagged.
///
/// ```
/// use casement_core::geometry::{PhysicalRect, PhysicalSize};
/// use casement_render::pass::{ThumbLayer, compose_thumb};
///
/// let red = [255, 0, 0, 255].repeat(4);
/// let layer = ThumbLayer::new(
///     PhysicalRect { x: 1, y: 0, w: 1, h: 1 },
///     PhysicalSize { w: 2, h: 2 },
///     &red,
/// );
/// let picture = compose_thumb(PhysicalSize { w: 2, h: 1 }, &[layer]);
/// assert_eq!(picture, [0, 0, 0, 0, 255, 0, 0, 255]);
/// ```
pub fn compose_thumb(size: PhysicalSize, layers: &[ThumbLayer<'_>]) -> Vec<u8> {
    let (w, h) = (size.w as usize, size.h as usize);
    let mut picture = vec![0_u8; w.saturating_mul(h).saturating_mul(4)];
    for layer in layers {
        draw(&mut picture, size, layer);
    }
    picture
}

/// Draws one layer over the picture.
fn draw(picture: &mut [u8], size: PhysicalSize, layer: &ThumbLayer<'_>) {
    let dest = layer.dest;
    let (src_w, src_h) = (u64::from(layer.src.w), u64::from(layer.src.h));
    let wanted = usize::try_from(src_w * src_h * 4).unwrap_or(usize::MAX);
    if dest.w == 0 || dest.h == 0 || src_w == 0 || src_h == 0 || layer.rgba.len() < wanted {
        return;
    }
    let x_range = clipped(dest.x, dest.w, size.w);
    let y_range = clipped(dest.y, dest.h, size.h);
    for py in y_range {
        let row = u64::try_from(i64::from(py) - i64::from(dest.y)).unwrap_or(0);
        let (y0, y1) = span(row, u64::from(dest.h), src_h);
        for px in x_range.clone() {
            let column = u64::try_from(i64::from(px) - i64::from(dest.x)).unwrap_or(0);
            let (x0, x1) = span(column, u64::from(dest.w), src_w);
            let covered = average(layer.rgba, src_w, (x0, x1), (y0, y1));
            let at = (py as usize * size.w as usize + px as usize) * 4;
            if let Some(under) = picture.get_mut(at..at + 4) {
                over(under, covered);
            }
        }
    }
}

/// The picture columns (or rows) `origin..origin + length` covers within `0..limit`.
fn clipped(origin: i32, length: u32, limit: u32) -> std::ops::Range<u32> {
    let start = i64::from(origin).max(0);
    let end = (i64::from(origin) + i64::from(length)).min(i64::from(limit));
    let (start, end) = (
        u32::try_from(start).unwrap_or(0),
        u32::try_from(end).unwrap_or(0),
    );
    start..end.max(start)
}

/// The window pixels `[from, to)` that picture pixel `index` of `dest` pixels covers, out of
/// `src` window pixels: at least one.
fn span(index: u64, dest: u64, src: u64) -> (u64, u64) {
    let from = (index * src / dest).min(src - 1);
    let to = (((index + 1) * src).div_ceil(dest)).clamp(from + 1, src);
    (from, to)
}

/// The mean of the window pixels in the box, per channel.
fn average(rgba: &[u8], width: u64, (x0, x1): (u64, u64), (y0, y1): (u64, u64)) -> [u8; 4] {
    let mut sums = [0_u64; 4];
    for y in y0..y1 {
        for x in x0..x1 {
            let at = usize::try_from((y * width + x) * 4).unwrap_or(0);
            if let Some(pixel) = rgba.get(at..at + 4) {
                for (sum, channel) in sums.iter_mut().zip(pixel) {
                    *sum += u64::from(*channel);
                }
            }
        }
    }
    let count = ((x1 - x0) * (y1 - y0)).max(1);
    sums.map(|sum| u8::try_from(sum / count).unwrap_or(u8::MAX))
}

/// `above` (premultiplied) drawn over `under`.
fn over(under: &mut [u8], above: [u8; 4]) {
    let keep = 255 - u16::from(above[3]);
    for (low, high) in under.iter_mut().zip(above) {
        let mixed = u16::from(high) + (u16::from(*low) * keep + 127) / 255;
        *low = u8::try_from(mixed.min(255)).unwrap_or(u8::MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalPoint, LogicalSize};

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    fn pixel(picture: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * width + x) * 4) as usize;
        [
            picture[at],
            picture[at + 1],
            picture[at + 2],
            picture[at + 3],
        ]
    }

    #[test]
    fn a_window_lands_where_it_sits_on_the_output() {
        let output = rect(1920, 0, 1920, 1080);
        let picture = PhysicalSize { w: 192, h: 108 };
        // name, window, x, y, w, h
        let cases = [
            (
                "the whole output",
                rect(1920, 0, 1920, 1080),
                (0, 0, 192, 108),
            ),
            ("the right half", rect(2880, 0, 960, 1080), (96, 0, 96, 108)),
            (
                "a window reaching left",
                rect(1820, 100, 400, 200),
                (-10, 10, 40, 20),
            ),
            ("touching windows touch", rect(2000, 0, 5, 5), (8, 0, 1, 1)),
        ];
        for (name, window, (x, y, w, h)) in cases {
            let dest = thumb_dest(window, output, picture);
            assert_eq!((dest.x, dest.y, dest.w, dest.h), (x, y, w, h), "{name}");
        }
    }

    #[test]
    fn a_shrunk_window_averages_the_pixels_it_covers() {
        // 2x2 window: red, green over blue, white; shrunk to one pixel.
        let pixels = [
            255, 0, 0, 255, 0, 255, 0, 255, //
            0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let layer = ThumbLayer::new(
            PhysicalRect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            PhysicalSize { w: 2, h: 2 },
            &pixels,
        );
        let picture = compose_thumb(PhysicalSize { w: 1, h: 1 }, &[layer]);
        assert_eq!(picture, [127, 127, 127, 255]);
    }

    #[test]
    fn a_window_in_front_covers_one_behind_and_translucent_ones_blend() {
        let back = [200, 0, 0, 255].repeat(4);
        let front = [0, 0, 100, 100].repeat(4);
        let at = PhysicalRect {
            x: 0,
            y: 0,
            w: 2,
            h: 2,
        };
        let size = PhysicalSize { w: 2, h: 2 };
        let layers = [
            ThumbLayer::new(at, size, &back),
            ThumbLayer::new(at, size, &front),
        ];
        let picture = compose_thumb(size, &layers);
        // 200 * (255 - 100) / 255 rounds to 122; the front adds its own 100 of blue.
        assert_eq!(pixel(&picture, 2, 1, 1), [122, 0, 100, 255]);
    }

    #[test]
    fn a_layer_outside_the_picture_or_with_too_few_pixels_draws_nothing() {
        let pixels = [255, 255, 255, 255].repeat(4);
        let size = PhysicalSize { w: 2, h: 2 };
        let away = PhysicalRect {
            x: 5,
            y: 5,
            w: 2,
            h: 2,
        };
        let nowhere = ThumbLayer::new(away, size, &pixels);
        let short = ThumbLayer::new(
            PhysicalRect {
                x: 0,
                y: 0,
                w: 2,
                h: 2,
            },
            size,
            &pixels[..8],
        );
        assert!(
            compose_thumb(size, &[nowhere, short])
                .iter()
                .all(|b| *b == 0)
        );
    }

    #[test]
    fn a_window_half_off_the_picture_keeps_the_scale() {
        // A 4 pixel wide window drawn 4 wide at x = -2: the picture holds its right half.
        let mut pixels = Vec::new();
        for column in 0..4_u8 {
            pixels.extend_from_slice(&[column * 10 + 1, 0, 0, 255]);
        }
        let layer = ThumbLayer::new(
            PhysicalRect {
                x: -2,
                y: 0,
                w: 4,
                h: 1,
            },
            PhysicalSize { w: 4, h: 1 },
            &pixels,
        );
        let picture = compose_thumb(PhysicalSize { w: 2, h: 1 }, &[layer]);
        assert_eq!(pixel(&picture, 2, 0, 0)[0], 21);
        assert_eq!(pixel(&picture, 2, 1, 0)[0], 31);
    }
}
