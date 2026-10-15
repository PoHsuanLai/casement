//! The colour pass: sRGB content converted to the panel's gamut, on the CPU (pixman, tests, and
//! the reference the shader in `color_gles` follows). The pass is skipped when the output's gamut
//! is sRGB ([`ColorTransform::for_gamut`] gives none), so a normal panel pays nothing.
//!
//! Per pixel: undo the source transfer function, multiply by the matrix in linear light, encode
//! with the sRGB curve (a panel is driven as sRGB), clamp to the panel's range.

use super::color_matrix::{ColorMatrix, srgb_to_panel};
use casement_core::output::Gamut;

/// How a surface's values map to light.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Transfer {
    /// The sRGB piecewise curve (untagged surfaces, `srgb`).
    #[default]
    Srgb,
    /// A pure power of 2.2.
    Gamma22,
    /// Already linear light (`ext_linear`).
    Linear,
}

impl Transfer {
    /// The shader's selector for this curve.
    pub fn selector(self) -> f32 {
        match self {
            Transfer::Srgb => 0.0,
            Transfer::Gamma22 => 1.0,
            Transfer::Linear => 2.0,
        }
    }

    /// Light from a value in 0..=1.
    pub fn decode(self, value: f64) -> f64 {
        match self {
            Transfer::Srgb if value <= 0.04045 => value / 12.92,
            Transfer::Srgb => ((value + 0.055) / 1.055).powf(2.4),
            Transfer::Gamma22 => value.max(0.0).powf(2.2),
            Transfer::Linear => value,
        }
    }
}

/// The sRGB encoding of light.
fn encode(light: f64) -> f64 {
    let light = light.clamp(0.0, 1.0);
    match light <= 0.003_130_8 {
        true => light * 12.92,
        false => 1.055 * light.powf(1.0 / 2.4) - 0.055,
    }
}

/// A conversion of a surface's colours to an output's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorTransform {
    /// Linear source RGB to linear panel RGB.
    pub matrix: ColorMatrix,
    /// The surface's transfer function.
    pub source: Transfer,
}

impl ColorTransform {
    /// The transform for sRGB content on a panel of `gamut`; none when nothing would change.
    ///
    /// ```
    /// use casement_core::output::{DISPLAY_P3, Gamut};
    /// use casement_render::pass::ColorTransform;
    ///
    /// assert!(ColorTransform::for_gamut(Gamut::Srgb).is_none());
    /// assert!(ColorTransform::for_gamut(Gamut::Wide(DISPLAY_P3)).is_some());
    /// ```
    pub fn for_gamut(gamut: Gamut) -> Option<ColorTransform> {
        match gamut {
            Gamut::Srgb => None,
            wide => Some(ColorTransform {
                matrix: srgb_to_panel(wide),
                source: Transfer::Srgb,
            }),
        }
    }

    /// The same conversion for a surface tagged with another transfer function.
    pub fn with_source(self, source: Transfer) -> ColorTransform {
        ColorTransform { source, ..self }
    }

    /// One colour (straight, not premultiplied; 0..=1 per channel) converted.
    pub fn apply(&self, rgb: [f64; 3]) -> [f64; 3] {
        self.matrix
            .apply(rgb.map(|c| self.source.decode(c)))
            .map(encode)
    }

    /// Premultiplied 8-bit RGBA pixels (`pixels` is groups of four) converted in place.
    pub fn apply_premultiplied(&self, pixels: &mut [u8]) {
        for pixel in pixels.chunks_exact_mut(4) {
            let alpha = f64::from(pixel[3]) / 255.0;
            if alpha <= 0.0 {
                continue;
            }
            let straight = [pixel[0], pixel[1], pixel[2]].map(|c| f64::from(c) / 255.0 / alpha);
            let converted = self.apply(straight.map(|c| c.min(1.0)));
            for (slot, value) in pixel.iter_mut().zip(converted) {
                *slot = (value * alpha * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::output::DISPLAY_P3;

    fn p3() -> ColorTransform {
        ColorTransform::for_gamut(Gamut::Wide(DISPLAY_P3)).expect("a transform")
    }

    #[test]
    fn an_srgb_panel_is_skipped() {
        assert_eq!(ColorTransform::for_gamut(Gamut::Srgb), None);
    }

    #[test]
    fn greys_stay_grey_and_the_ends_stay_put() {
        for grey in [0.0, 0.1, 0.5, 0.9, 1.0] {
            let out = p3().apply([grey; 3]);
            assert!(
                out.iter().all(|c| (c - grey).abs() < 2e-3),
                "{grey}: {out:?}"
            );
        }
    }

    #[test]
    fn saturated_red_is_pulled_in() {
        // Linear sRGB red in P3 is (0.8225, 0.0332, 0.0171): less red, a little of the others.
        let out = p3().apply([1.0, 0.0, 0.0]);
        assert!(out[0] < 0.95 && out[1] > 0.1 && out[2] > 0.05, "{out:?}");
    }

    #[test]
    fn transfer_curves_decode_known_points() {
        let rows = [
            (Transfer::Srgb, 0.5, 0.2140),
            (Transfer::Gamma22, 0.5, 0.2176),
            (Transfer::Linear, 0.5, 0.5),
            (Transfer::Srgb, 1.0, 1.0),
        ];
        for (transfer, value, want) in rows {
            assert!(
                (transfer.decode(value) - want).abs() < 1e-3,
                "{transfer:?} {value}"
            );
        }
    }

    #[test]
    fn linear_sources_are_brighter_than_srgb_ones() {
        let srgb = p3().apply([0.2; 3]);
        let linear = p3().with_source(Transfer::Linear).apply([0.2; 3]);
        assert!(linear[0] > srgb[0]);
    }

    #[test]
    fn premultiplied_pixels_convert_and_clear_ones_are_left() {
        let mut pixels = [255, 0, 0, 255, 0, 0, 0, 0, 128, 0, 0, 128];
        p3().apply_premultiplied(&mut pixels);
        assert!(pixels[0] < 250 && pixels[1] > 20);
        assert_eq!(pixels[4..8], [0, 0, 0, 0]);
        assert!(pixels[8] < 128 && pixels[9] > 5);
    }
}
