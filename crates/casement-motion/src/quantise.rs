//! The one float-to-unit step.

use casement_core::geometry::SubPx;

/// `x` logical pixels in whole sub-pixels, rounded half away from zero; saturating at `i32`'s
/// range, and 0 for NaN.
pub fn quantise(x: f64) -> SubPx {
    SubPx((x * 120.0).round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_land_on_whole_sub_pixels() {
        const CASES: &[(&str, f64, i32)] = &[
            ("zero", 0.0, 0),
            ("one pixel", 1.0, 120),
            ("a quarter", 1.25, 150),
            ("negative", -0.75, -90),
            ("half a sub-pixel rounds away", 1.0 / 240.0, 1),
            ("negative half rounds away", -1.0 / 240.0, -1),
            ("below half", 0.004, 0),
            ("too large saturates", 1e12, i32::MAX),
            ("too small saturates", -1e12, i32::MIN),
            ("not a number", f64::NAN, 0),
        ];
        for (name, x, want) in CASES {
            assert_eq!(quantise(*x), SubPx(*want), "{name}");
        }
    }
}
