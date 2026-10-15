//! How a surface's colours are encoded, as a client tells it (`wp_color_manager_v1`): the
//! primaries and the transfer curve. Integers only; the maths is in `casement-render`.

use super::gamut::{Primaries, SRGB};

/// The transfer functions casement understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum TransferCurve {
    /// The sRGB piecewise curve.
    #[default]
    Srgb,
    /// A pure power of 2.2.
    Gamma22,
    /// Linear light.
    Linear,
}

/// A surface's colour encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColorTag {
    /// The primaries (white is D65).
    pub primaries: Primaries,
    /// The transfer curve.
    pub transfer: TransferCurve,
}

impl ColorTag {
    /// What an untagged surface is taken to be.
    pub const SRGB: ColorTag = ColorTag {
        primaries: SRGB,
        transfer: TransferCurve::Srgb,
    };
}
