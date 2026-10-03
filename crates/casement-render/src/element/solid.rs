//! Solid rectangles, with colours that reach the renderer already quantised (A18).

use casement_core::geometry::PhysicalRect;
use smithay::backend::renderer::Color32F;
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::utils::CommitCounter;

/// A colour in whole steps of 1/255, premultiplied: what every colour token becomes before it
/// reaches a renderer, so pixman and GLES agree to the byte (SP2, A18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba8 {
    /// Red, premultiplied.
    pub r: u8,
    /// Green, premultiplied.
    pub g: u8,
    /// Blue, premultiplied.
    pub b: u8,
    /// Alpha.
    pub a: u8,
}

impl Rgba8 {
    /// smithay's float colour, each channel exactly `n / 255`.
    pub fn color32f(self) -> Color32F {
        let unit = |n: u8| f32::from(n) / 255.0;
        Color32F::new(unit(self.r), unit(self.g), unit(self.b), unit(self.a))
    }
}

/// A solid element covering `rect` (output-local physical pixels) in `colour`; `id` and
/// `commit` keep its damage stable across frames.
pub fn solid_rect(
    id: Id,
    rect: PhysicalRect,
    colour: Rgba8,
    commit: CommitCounter,
) -> SolidColorRenderElement {
    let _ = (id, rect, colour, commit);
    todo!("P0-R-scene: solid_rect")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantised_colours_are_exact_steps_of_one_255th() {
        let colour = Rgba8 {
            r: 0,
            g: 26,
            b: 255,
            a: 128,
        }
        .color32f();
        assert_eq!(colour.r(), 0.0);
        assert_eq!(colour.g(), 26.0 / 255.0);
        assert_eq!(colour.b(), 1.0);
        assert_eq!(colour.a(), 128.0 / 255.0);
    }
}
