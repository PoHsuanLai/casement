//! Sub-pixel units: 1/120 of a logical pixel, the unit motion samples into so a frame is `Eq`
//! data and a motion golden is text (120 because it is the wire's scale denominator, so every
//! scale a compositor announces lands on whole device pixels at whole sub-pixels).

use serde::{Deserialize, Serialize};

/// Sub-pixels per logical pixel.
const PER_LOGICAL: i64 = 120;

/// A length or coordinate in 1/120 logical pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SubPx(pub i32);

impl SubPx {
    /// `px` whole logical pixels.
    pub fn of_logical(px: i32) -> SubPx {
        SubPx(saturate(i64::from(px) * PER_LOGICAL))
    }

    /// The nearest whole logical pixel, halves away from zero.
    pub fn round_logical(self) -> i32 {
        let value = i64::from(self.0);
        let half = PER_LOGICAL / 2;
        let rounded = if value >= 0 {
            (value + half) / PER_LOGICAL
        } else {
            (value - half) / PER_LOGICAL
        };
        saturate(rounded)
    }
}

/// `value` held to `i32`'s range.
fn saturate(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

/// A point in sub-pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubPoint {
    /// Rightward.
    pub x: SubPx,
    /// Downward.
    pub y: SubPx,
}

/// A size in sub-pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubSize {
    /// The width.
    pub w: SubPx,
    /// The height.
    pub h: SubPx,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_pixels_round_trip_and_round_halves_away_from_zero() {
        const CASES: &[(&str, i32, i32)] = &[
            ("zero", 0, 0),
            ("whole", 240, 2),
            ("below half", 59, 0),
            ("half", 60, 1),
            ("negative half", -60, -1),
            ("negative below half", -59, 0),
            ("saturates", i32::MAX, 17_895_697),
        ];
        for (name, sub, want) in CASES {
            assert_eq!(SubPx(*sub).round_logical(), *want, "{name}");
        }
        assert_eq!(SubPx::of_logical(-3), SubPx(-360));
        assert_eq!(SubPx::of_logical(i32::MAX), SubPx(i32::MAX));
        assert_eq!(SubPx::of_logical(17).round_logical(), 17);
    }
}
