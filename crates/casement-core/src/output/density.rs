//! A display's density, in integers: pixels per inch from the preferred mode and the EDID's
//! millimetres. Floats never enter the model, so the square roots are integer roots of scaled
//! squares (accurate to a thousandth of an inch, far finer than the policy's rounding).

use super::PhysicalMm;
use crate::geometry::PhysicalSize;

/// Millimetres in an inch, times a thousand.
const MILLI_MM_PER_INCH: u64 = 25_400;
/// A density outside this many pixels per inch is an EDID that got its size wrong (a TV that
/// reports 16 x 9 cm, a placeholder), not a display: the density is unknown. 600 clears the
/// densest phone panels Linux runs on (about 560).
const PLAUSIBLE: std::ops::RangeInclusive<u64> = 40..=600;

/// Pixels per inch, times a thousand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MilliPpi(pub u64);

/// The density of `pixels` over `mm`, along the diagonal; `None` when the size is missing or
/// implausible.
pub(super) fn density(pixels: PhysicalSize, mm: PhysicalMm) -> Option<MilliPpi> {
    let diagonal_px = milli_diagonal(u64::from(pixels.w), u64::from(pixels.h));
    let diagonal_mm = milli_diagonal(u64::from(mm.w), u64::from(mm.h));
    // ppi = px * 25.4 / mm; both diagonals carry a factor of 1000 that cancels, and 25 400 is
    // 25.4 times the 1000 the result keeps.
    let milli_ppi = diagonal_px.checked_mul(MILLI_MM_PER_INCH)? / diagonal_mm.max(1);
    let plausible = diagonal_mm > 0 && PLAUSIBLE.contains(&((milli_ppi + 500) / 1000));
    plausible.then_some(MilliPpi(milli_ppi))
}

/// `sqrt(a² + b²) × 1000`, rounded down.
fn milli_diagonal(a: u64, b: u64) -> u64 {
    let squares = a.saturating_mul(a).saturating_add(b.saturating_mul(b));
    squares.saturating_mul(1_000_000).isqrt()
}
