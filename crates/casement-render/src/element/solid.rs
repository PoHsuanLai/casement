//! Solid rectangles, with colours that reach the renderer already quantised (A18).

use crate::scene::convert::physical;
use casement_core::geometry::PhysicalRect;
use smithay::backend::renderer::Color32F;
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::{Id, Kind};
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

impl Rgba8 {
    /// The colour at `alpha` of its own strength (premultiplied, so every channel scales).
    ///
    /// ```
    /// use casement_render::Rgba8;
    ///
    /// let white = Rgba8 { r: 255, g: 255, b: 255, a: 255 };
    /// assert_eq!(white.faded(0.5), Rgba8 { r: 255, g: 255, b: 255, a: 255 }.color32f() * 0.5);
    /// ```
    pub fn faded(self, alpha: f32) -> Color32F {
        let unit = |n: u8| f32::from(n) / 255.0 * alpha;
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
    SolidColorRenderElement::new(
        id,
        physical(rect),
        commit,
        colour.color32f(),
        Kind::Unspecified,
    )
}

impl Rgba8 {
    /// The same colour with full alpha: what an opaque-by-rule cover draws.
    pub(crate) fn opaque(self) -> Rgba8 {
        Rgba8 { a: 255, ..self }
    }

    /// Whether nothing shows through.
    pub(crate) fn is_opaque(self) -> bool {
        self.a == 255
    }
}

/// A commit counter that names content rather than counting changes: FNV-1a over `bytes`.
/// Elements whose content is a pure function of a few values (a colour, a region) use it, so
/// equal content gives an equal counter and the damage tracker redraws nothing.
pub(crate) fn fingerprint(bytes: impl IntoIterator<Item = u8>) -> CommitCounter {
    let hash = bytes
        .into_iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    // Folding to the platform's width keeps the fingerprint deterministic on 32-bit targets.
    let folded = (hash ^ (hash >> 32)) as usize;
    CommitCounter::from(folded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_solid_rect_covers_its_rectangle_in_its_colour() {
        use smithay::backend::renderer::element::Element;
        use smithay::utils::Scale;
        let rect = PhysicalRect {
            x: -4,
            y: 7,
            w: 30,
            h: 20,
        };
        let colour = Rgba8 {
            r: 10,
            g: 20,
            b: 30,
            a: 255,
        };
        let element = solid_rect(Id::new(), rect, colour, CommitCounter::from(3));
        assert_eq!(element.geometry(Scale::from(1.5)), physical(rect));
        assert_eq!(element.current_commit(), CommitCounter::from(3));
        assert_eq!(
            crate::scene::convert::ours(element.geometry(Scale::from(1.0))),
            rect
        );
    }

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
