//! A layer-shell surface as elements.

use super::CasementElement;
use super::blur_v0;
use crate::compositor::Compositor;
use crate::pass::BlurLook;
use crate::scene::OutputView;
use crate::scene::convert::{on_output, sub_of};
use crate::texture_book::TextureBook;
use casement_core::LayerKey;
use casement_core::geometry::LogicalRect;
use casement_core::layer::BlurStrength;
use ds_core::vocab::Fraction;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::render_elements_from_surface_tree;

/// `layer`'s surface tree at `area` (global logical) on `view`, front first, with its blur v0
/// behind it when it asked for one, as strong as `strength` says. Empty when the book has no
/// surface for it.
pub fn layer_elements<R: Compositor>(
    layer: &LayerKey,
    area: LogicalRect,
    strength: BlurStrength,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    layer_elements_faded(layer, area, strength, Fraction(1000), view, book, renderer)
}

/// [`layer_elements`] drawn at `opacity` (thousandths): a layer that cross-fades in or out. Its
/// blur fades with it.
pub fn layer_elements_faded<R: Compositor>(
    layer: &LayerKey,
    area: LogicalRect,
    strength: BlurStrength,
    opacity: Fraction,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let Some(entry) = book.layer(layer) else {
        return Vec::new();
    };
    let origin = sub_of(area.origin);
    let alpha = f32::from(opacity.clamped().0) / 1000.0;
    let mut elements: Vec<CasementElement<R>> = render_elements_from_surface_tree(
        renderer,
        &entry.root,
        on_output(origin, view),
        view.scale.as_f64(),
        alpha,
        Kind::Unspecified,
    );
    let look = BlurLook::of(strength);
    let blur = blur_v0::behind_scaled(&entry.root, origin, (1.0, 1.0), (alpha, look), view);
    elements.extend(blur.map(CasementElement::Blur));
    elements
}
