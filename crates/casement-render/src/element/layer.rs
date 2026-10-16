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
    let Some(entry) = book.layer(layer) else {
        return Vec::new();
    };
    let origin = sub_of(area.origin);
    let mut elements: Vec<CasementElement<R>> = render_elements_from_surface_tree(
        renderer,
        &entry.root,
        on_output(origin, view),
        view.scale.as_f64(),
        1.0,
        Kind::Unspecified,
    );
    let look = BlurLook::of(strength);
    elements.extend(blur_v0::behind(&entry.root, origin, look, view).map(CasementElement::Blur));
    elements
}
