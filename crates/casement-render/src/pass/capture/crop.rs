//! Cutting a part out of a captured frame (a region capture).

use super::CapturedFrame;
use casement_core::geometry::PhysicalRect;

impl CapturedFrame {
    /// The part of the frame inside `rect` (in pixels from the frame's top-left corner), as a
    /// frame of its own; none when `rect` reaches outside the frame or the bytes are short.
    ///
    /// ```
    /// use casement_core::geometry::PhysicalRect;
    /// use casement_render::pass::CapturedFrame;
    ///
    /// let whole = PhysicalRect { x: 0, y: 0, w: 2, h: 1 };
    /// let frame = CapturedFrame::from_rgba(whole, &[1, 2, 3, 4, 5, 6, 7, 8]);
    /// let right = PhysicalRect { x: 1, y: 0, w: 1, h: 1 };
    /// assert_eq!(frame.cropped(right).expect("inside").bytes, [7, 6, 5, 8]);
    /// ```
    pub fn cropped(&self, rect: PhysicalRect) -> Option<CapturedFrame> {
        let from_i32 = |n: i32| usize::try_from(n).ok();
        let from_u32 = |n: u32| usize::try_from(n).ok();
        let (x, y) = (from_i32(rect.x)?, from_i32(rect.y)?);
        let (w, h) = (from_u32(rect.w)?, from_u32(rect.h)?);
        let stride = from_u32(self.stride)?;
        if x.saturating_add(w) > from_u32(self.area.w)?
            || y.saturating_add(h) > from_u32(self.area.h)?
        {
            return None;
        }
        let row = w.checked_mul(4)?;
        let mut bytes = Vec::with_capacity(row.checked_mul(h)?);
        for line in y..y.saturating_add(h) {
            let start = line.checked_mul(stride)?.checked_add(x.checked_mul(4)?)?;
            bytes.extend_from_slice(self.bytes.get(start..start.checked_add(row)?)?);
        }
        Some(CapturedFrame {
            area: rect,
            format: self.format,
            stride: u32::try_from(row).ok()?,
            bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> CapturedFrame {
        let area = PhysicalRect {
            x: 0,
            y: 0,
            w: 3,
            h: 2,
        };
        let rgba: Vec<u8> = (0..24).collect();
        CapturedFrame::from_rgba(area, &rgba)
    }

    #[test]
    fn a_crop_keeps_the_rows_it_names() {
        let rect = PhysicalRect {
            x: 1,
            y: 1,
            w: 2,
            h: 1,
        };
        let cut = frame().cropped(rect).expect("inside");
        assert_eq!((cut.area, cut.stride), (rect, 8));
        assert_eq!(cut.bytes, frame().bytes[16..24]);
    }

    #[test]
    fn a_crop_outside_the_frame_is_none() {
        let table = [(2, 0, 2, 1), (0, 1, 1, 2), (-1, 0, 1, 1)];
        for (x, y, w, h) in table {
            let rect = PhysicalRect { x, y, w, h };
            assert!(frame().cropped(rect).is_none(), "{rect:?}");
        }
    }
}
