//! The region of `zwlr_screencopy_manager_v1.capture_output_region`: output-local logical
//! pixels turned into the physical pixels of the frame, cut to the output.

use casement_core::geometry::{PhysicalRect, PhysicalSize};
use ds_core::geometry::scale::Scale;

/// The part of the output's frame the logical rectangle `(x, y, w, h)` covers at `scale`, each
/// edge rounded to the nearest physical pixel and the whole cut to a frame of `mode`. `None`
/// when nothing of it lies on the output (or the rectangle has no area).
///
/// ```
/// use casement_core::geometry::PhysicalSize;
/// use casement_wayland::screencopy::physical_region;
/// use ds_core::geometry::scale::Scale;
///
/// let mode = PhysicalSize { w: 1920, h: 1080 };
/// let rect = physical_region((10, 10, 100, 50), Scale(240), mode).expect("on the output");
/// assert_eq!((rect.x, rect.y, rect.w, rect.h), (20, 20, 200, 100));
/// ```
pub fn physical_region(
    (x, y, w, h): (i32, i32, i32, i32),
    scale: Scale,
    mode: PhysicalSize,
) -> Option<PhysicalRect> {
    if w <= 0 || h <= 0 {
        return None;
    }
    let numerator = i64::from(scale.numerator());
    let denominator = i64::from(Scale::DENOMINATOR);
    let physical = |logical: i64| {
        logical
            .saturating_mul(numerator)
            .saturating_add(denominator / 2)
            .div_euclid(denominator)
    };
    let left = physical(i64::from(x)).max(0);
    let top = physical(i64::from(y)).max(0);
    let right = physical(i64::from(x).saturating_add(i64::from(w))).min(i64::from(mode.w));
    let bottom = physical(i64::from(y).saturating_add(i64::from(h))).min(i64::from(mode.h));
    if right <= left || bottom <= top {
        return None;
    }
    Some(PhysicalRect {
        x: i32::try_from(left).ok()?,
        y: i32::try_from(top).ok()?,
        w: u32::try_from(right - left).ok()?,
        h: u32::try_from(bottom - top).ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODE: PhysicalSize = PhysicalSize { w: 640, h: 480 };

    fn region(rect: (i32, i32, i32, i32), scale: u32) -> Option<(i32, i32, u32, u32)> {
        physical_region(rect, Scale(scale), MODE).map(|r| (r.x, r.y, r.w, r.h))
    }

    #[test]
    fn a_region_maps_through_the_scale_and_is_cut_to_the_output() {
        let table = [
            ((10, 20, 30, 40), 120, Some((10, 20, 30, 40))),
            ((10, 20, 30, 40), 240, Some((20, 40, 60, 80))),
            // 1.5x: edges 15 -> 22.5 -> 23 and 45 -> 67.5 -> 68.
            ((10, 10, 20, 20), 180, Some((15, 15, 30, 30))),
            ((-10, -10, 30, 30), 120, Some((0, 0, 20, 20))),
            ((600, 450, 100, 100), 120, Some((600, 450, 40, 30))),
            ((640, 0, 10, 10), 120, None),
            ((0, 0, 0, 10), 120, None),
            ((0, 0, 10, -1), 120, None),
            ((-50, 0, 40, 10), 120, None),
            // A one-pixel pick (a colour picker's `capture_output_region` of 1 x 1): one pixel at
            // 1x, the square the logical pixel covers at 2x, one rounded pixel at 1.5x, and the
            // last pixel of the output still maps.
            ((5, 5, 1, 1), 120, Some((5, 5, 1, 1))),
            ((5, 5, 1, 1), 240, Some((10, 10, 2, 2))),
            ((5, 5, 1, 1), 180, Some((8, 8, 1, 1))),
            ((639, 479, 1, 1), 120, Some((639, 479, 1, 1))),
            ((640, 479, 1, 1), 120, None),
        ];
        for (rect, scale, expected) in table {
            assert_eq!(region(rect, scale), expected, "{rect:?} at {scale}");
        }
    }
}
