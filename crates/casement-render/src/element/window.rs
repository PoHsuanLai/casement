//! A window's surface tree as elements.

use super::CasementElement;
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::texture_book::TextureBook;
use casement_motion::PlacedWindow;

/// `placed`'s surface tree (subsurfaces, then the root) drawn at its sampled place, size and
/// opacity on `view`, front first. Its committed buffer is scaled to the sampled size until it
/// commits the new one. Empty when the book has no surface for it.
pub fn window_elements<R: Compositor>(
    placed: &PlacedWindow,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let _ = (placed, view, book, renderer);
    todo!("P0-R-scene: window_elements")
}
