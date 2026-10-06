//! A window's surface tree as elements.

use super::CasementElement;
use super::blur_v0;
use super::geometry::surface_origin;
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::scene::convert::on_output;
use crate::texture_book::TextureBook;
use casement_motion::PlacedWindow;
use ds_core::vocab::Fraction;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::render_elements_from_surface_tree;

/// `placed`'s surface tree (subsurfaces, then the root) drawn at its sampled place and opacity
/// on `view`, front first, with its blur v0 behind it when it asked for one. Empty when the
/// book has no surface for it.
///
/// The tree is drawn so that the root's window geometry origin lands at the sampled place
/// ([`surface_origin`]), at its committed size: the closed element set has no rescaling wrapper,
/// so a sampled size that differs from the committed one (a resize in flight) shows once the
/// client commits it (FINDINGS: P3-Z adds the wrapper with the effect elements).
pub fn window_elements<R: Compositor>(
    placed: &PlacedWindow,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let Some(entry) = book.window(&placed.window) else {
        return Vec::new();
    };
    let origin = surface_origin(&entry.root, placed.at);
    let mut elements: Vec<CasementElement<R>> = render_elements_from_surface_tree(
        renderer,
        &entry.root,
        on_output(origin, view),
        view.scale.as_f64(),
        alpha(placed.opacity),
        Kind::Unspecified,
    );
    elements.extend(blur_v0::behind(&entry.root, origin, view).map(CasementElement::Blur));
    elements
}

/// An opacity as smithay's alpha: thousandths, at most one.
pub(crate) fn alpha(opacity: Fraction) -> f32 {
    f32::from(opacity.0.min(1000)) / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_becomes_alpha_held_to_one() {
        assert_eq!(alpha(Fraction(0)), 0.0);
        assert_eq!(alpha(Fraction(350)), 0.35);
        assert_eq!(alpha(Fraction(1000)), 1.0);
        assert_eq!(alpha(Fraction(1400)), 1.0);
    }
}
