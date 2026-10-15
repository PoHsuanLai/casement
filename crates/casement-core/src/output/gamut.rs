//! The colours an output's panel can show: its red, green and blue primaries as the EDID
//! reports them (CIE 1931 xy), and whether that is wider than sRGB. Integers only (ten-thousandths);
//! the maths that turns them into a matrix is in `casement-render`'s colour pass.

/// A point on the CIE 1931 xy chromaticity diagram, in ten-thousandths (0.6400 is 6400).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chromaticity {
    /// The x coordinate.
    pub x: u16,
    /// The y coordinate.
    pub y: u16,
}

impl Chromaticity {
    const fn at(x: u16, y: u16) -> Chromaticity {
        Chromaticity { x, y }
    }

    /// Whether both coordinates are within `tolerance` of `other`'s.
    fn near(self, other: Chromaticity, tolerance: u16) -> bool {
        self.x.abs_diff(other.x) <= tolerance && self.y.abs_diff(other.y) <= tolerance
    }
}

/// A display's three primaries (the white point is always D65 here: see [`D65`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Primaries {
    /// Red.
    pub red: Chromaticity,
    /// Green.
    pub green: Chromaticity,
    /// Blue.
    pub blue: Chromaticity,
}

/// The D65 white point sRGB and Display P3 share. A panel's own reported white is ignored: content
/// white stays the panel's native white, which is what a viewer expects of "white".
pub const D65: Chromaticity = Chromaticity::at(3127, 3290);

/// BT.709 / sRGB.
pub const SRGB: Primaries = Primaries {
    red: Chromaticity::at(6400, 3300),
    green: Chromaticity::at(3000, 6000),
    blue: Chromaticity::at(1500, 600),
};

/// Display P3 (D65).
pub const DISPLAY_P3: Primaries = Primaries {
    red: Chromaticity::at(6800, 3200),
    green: Chromaticity::at(2650, 6900),
    blue: Chromaticity::at(1500, 600),
};

/// How far a primary may sit from sRGB's and still count as sRGB: ten-thousandths. EDID rounds to
/// 1/1024 and panels are never exact.
const SRGB_TOLERANCE: u16 = 300;
const BASE_BLOCK: usize = 128;
const HEADER: [u8; 8] = [0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00];
/// Where the ten chromaticity bytes start in the base block.
const CHROMA_AT: usize = 25;

/// Whether an output needs a conversion from sRGB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gamut {
    /// About sRGB: sRGB content passes through untouched.
    Srgb,
    /// Wider than sRGB: sRGB content is converted so it does not look oversaturated.
    Wide(Primaries),
}

impl Gamut {
    /// The gamut of a panel with these primaries. A panel whose primaries are all near sRGB's
    /// (or inside it, like a 72% laptop panel is not distinguished from sRGB) reads as sRGB.
    ///
    /// ```
    /// use casement_core::output::{DISPLAY_P3, Gamut, SRGB};
    ///
    /// assert_eq!(Gamut::of(SRGB), Gamut::Srgb);
    /// assert_eq!(Gamut::of(DISPLAY_P3), Gamut::Wide(DISPLAY_P3));
    /// ```
    pub fn of(primaries: Primaries) -> Gamut {
        let near = primaries.red.near(SRGB.red, SRGB_TOLERANCE)
            && primaries.green.near(SRGB.green, SRGB_TOLERANCE)
            && primaries.blue.near(SRGB.blue, SRGB_TOLERANCE);
        match near {
            true => Gamut::Srgb,
            false => Gamut::Wide(primaries),
        }
    }
}

impl Primaries {
    /// The primaries an EDID base block reports; none when the block is not an EDID or reports
    /// all zeros (a virtual display).
    ///
    /// ```
    /// use casement_core::output::Primaries;
    ///
    /// assert_eq!(Primaries::from_edid(&[]), None);
    /// ```
    pub fn from_edid(bytes: &[u8]) -> Option<Primaries> {
        let block = bytes.get(..BASE_BLOCK)?;
        if block[..8] != HEADER {
            return None;
        }
        let low = [block[CHROMA_AT], block[CHROMA_AT + 1]];
        let high = &block[CHROMA_AT + 2..CHROMA_AT + 10];
        // Ten bits each: the high eight in their own byte, the low two packed four to a byte.
        let value = |index: usize| -> u32 {
            let shift = 6 - 2 * (index % 4) as u32;
            let packed = u32::from(low[index / 4] >> shift) & 0b11;
            (u32::from(high[index]) << 2) | packed
        };
        let point = |index: usize| Chromaticity {
            x: ten_thousandths(value(index)),
            y: ten_thousandths(value(index + 1)),
        };
        let primaries = Primaries {
            red: point(0),
            green: point(2),
            blue: point(4),
        };
        let silent = [primaries.red, primaries.green, primaries.blue]
            .iter()
            .all(|p| p.x == 0 && p.y == 0);
        (!silent).then_some(primaries)
    }
}

/// A ten-bit EDID coordinate (1/1024 steps) in ten-thousandths, rounded.
fn ten_thousandths(value: u32) -> u16 {
    u16::try_from((value * 10_000 + 512) / 1024).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A base block with a valid header and these ten-bit chromaticities (rx, ry, gx, gy, bx, by,
    /// wx, wy).
    fn block(values: [u16; 8]) -> Vec<u8> {
        let mut bytes = vec![0u8; BASE_BLOCK];
        bytes[..8].copy_from_slice(&HEADER);
        for (index, value) in values.iter().enumerate() {
            bytes[CHROMA_AT + 2 + index] = (value >> 2) as u8;
            bytes[CHROMA_AT + index / 4] |= ((value & 0b11) as u8) << (6 - 2 * (index % 4));
        }
        bytes
    }

    #[test]
    fn edid_chromaticities_come_out_in_ten_thousandths() {
        // sRGB as EDIDs write it: 0.640 is 655/1024, and so on.
        let edid = block([655, 338, 307, 614, 154, 61, 320, 337]);
        let primaries = Primaries::from_edid(&edid).expect("primaries");
        assert_eq!(primaries.red, Chromaticity::at(6396, 3301));
        assert_eq!(primaries.green, Chromaticity::at(2998, 5996));
        assert_eq!(primaries.blue, Chromaticity::at(1504, 596));
        assert_eq!(Gamut::of(primaries), Gamut::Srgb);
    }

    #[test]
    fn a_p3_panel_is_wide() {
        // 0.680 0.320 / 0.265 0.690 / 0.150 0.060.
        let edid = block([696, 328, 271, 707, 154, 61, 320, 337]);
        let primaries = Primaries::from_edid(&edid).expect("primaries");
        assert!(matches!(Gamut::of(primaries), Gamut::Wide(_)));
    }

    #[test]
    fn silent_and_foreign_blocks_report_nothing() {
        assert_eq!(Primaries::from_edid(&block([0; 8])), None);
        assert_eq!(Primaries::from_edid(&[0; BASE_BLOCK]), None);
        assert_eq!(Primaries::from_edid(&[0; 10]), None);
    }
}
