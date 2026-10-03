//! Test patterns: pixels whose values say where they are, so a misplaced, scaled or cropped
//! buffer shows in a golden or a pixel probe.

use casement_core::geometry::PhysicalSize;

/// A test pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pattern {
    /// Opaque white and black squares of `cell` pixels, white at the origin.
    Checker {
        /// The square's side.
        cell: u32,
    },
    /// Red rising left to right, green top to bottom, blue at half, opaque.
    Gradient,
    /// Each pixel's coordinates in its colour: red = x mod 256, green = y mod 256, blue = the
    /// high nibbles (x / 256 in the low nibble, y / 256 in the high), opaque.
    CoordinateEncoding,
    /// One ARGB colour.
    Solid(u32),
}

impl Pattern {
    /// The ARGB8888 value of pixel (`x`, `y`) of a `size` buffer.
    pub fn pixel(self, x: u32, y: u32, size: PhysicalSize) -> u32 {
        let argb =
            |r: u32, g: u32, b: u32| 0xff00_0000 | (r & 0xff) << 16 | (g & 0xff) << 8 | (b & 0xff);
        match self {
            Pattern::Checker { cell } => {
                let cell = cell.max(1);
                if (x / cell + y / cell).is_multiple_of(2) {
                    0xffff_ffff
                } else {
                    0xff00_0000
                }
            }
            Pattern::Gradient => {
                let ramp =
                    |at: u32, extent: u32| at.saturating_mul(255) / extent.saturating_sub(1).max(1);
                argb(ramp(x, size.w), ramp(y, size.h), 0x80)
            }
            Pattern::CoordinateEncoding => argb(x, y, (x >> 8 & 0xf) | (y >> 8 & 0xf) << 4),
            Pattern::Solid(argb) => argb,
        }
    }

    /// The whole buffer, rows top to bottom, ARGB8888 little-endian (what `wl_shm` calls
    /// `argb8888`).
    pub fn bytes(self, size: PhysicalSize) -> Vec<u8> {
        (0..size.h)
            .flat_map(|y| (0..size.w).map(move |x| (x, y)))
            .flat_map(|(x, y)| self.pixel(x, y, size).to_le_bytes())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: PhysicalSize = PhysicalSize { w: 300, h: 600 };

    #[test]
    fn the_checker_alternates_from_white() {
        let checker = Pattern::Checker { cell: 10 };
        assert_eq!(checker.pixel(0, 0, SIZE), 0xffff_ffff);
        assert_eq!(checker.pixel(10, 0, SIZE), 0xff00_0000);
        assert_eq!(checker.pixel(10, 10, SIZE), 0xffff_ffff);
        assert_eq!(Pattern::Checker { cell: 0 }.pixel(1, 0, SIZE), 0xff00_0000);
    }

    #[test]
    fn the_gradient_spans_each_axis() {
        assert_eq!(Pattern::Gradient.pixel(0, 0, SIZE), 0xff00_0080);
        assert_eq!(Pattern::Gradient.pixel(299, 599, SIZE), 0xffff_ff80);
    }

    #[test]
    fn coordinates_can_be_read_back_from_the_colour() {
        let p = Pattern::CoordinateEncoding.pixel(290, 513, SIZE);
        let (r, g, b) = (p >> 16 & 0xff, p >> 8 & 0xff, p & 0xff);
        assert_eq!(r | (b & 0xf) << 8, 290);
        assert_eq!(g | (b >> 4) << 8, 513);
    }

    #[test]
    fn a_buffer_is_four_little_endian_bytes_per_pixel() {
        let bytes = Pattern::Solid(0x8011_2233).bytes(PhysicalSize { w: 2, h: 1 });
        assert_eq!(bytes, [0x33, 0x22, 0x11, 0x80, 0x33, 0x22, 0x11, 0x80]);
    }
}
