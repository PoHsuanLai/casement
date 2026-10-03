//! A layer-shell surface as elements.

use super::CasementElement;
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::texture_book::TextureBook;
use casement_core::LayerKey;
use casement_core::geometry::LogicalRect;

/// `layer`'s surface tree at `area` (global logical) on `view`, front first. Empty when the
/// book has no surface for it.
pub fn layer_elements<R: Compositor>(
    layer: &LayerKey,
    area: LogicalRect,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let _ = (layer, area, view, book, renderer);
    todo!("P0-R-scene: layer_elements")
}
