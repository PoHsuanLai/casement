//! The cursor as elements.

use super::CasementElement;
use crate::compositor::Compositor;
use crate::scene::{ChromeLayer, OutputView};
use crate::texture_book::TextureBook;

/// The cursor `chrome` describes, on `view` (nothing when it is hidden or off this output).
/// Named shapes come from the cursor theme as a CPU raster at the output's native scale (A18).
pub fn cursor_elements<R: Compositor>(
    chrome: &ChromeLayer,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let _ = (chrome, view, book, renderer);
    todo!("P0-R-scene: cursor_elements")
}
