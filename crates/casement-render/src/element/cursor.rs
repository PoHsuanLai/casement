//! The cursor as elements.

mod face;
mod raster;

pub use face::{CursorFace, cursor_face, cursor_pixels};

use super::CasementElement;
use crate::compositor::Compositor;
use crate::scene::convert::{on_output, sub_of};
use crate::scene::{ChromeLayer, CursorDraw, CursorShape, OutputView};
use crate::texture_book::TextureBook;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use ds_core::geometry::scale::Scale;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::memory::{
    MemoryRenderBuffer, MemoryRenderBufferRenderElement,
};
use smithay::backend::renderer::element::surface::render_elements_from_surface_tree;
use smithay::utils::{Logical, Rectangle, Size, Transform};

/// The cursor `chrome` describes, on `view` (nothing when it is hidden or off this output).
/// Named shapes are casement's own rasters at the output's native scale (A18), drawn 1:1.
pub fn cursor_elements<R: Compositor>(
    chrome: &ChromeLayer,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    match chrome.cursor {
        CursorDraw::Hidden => Vec::new(),
        CursorDraw::Named(shape) => named(shape, chrome.cursor_at, view, book, renderer)
            .map(CasementElement::Memory)
            .into_iter()
            .collect(),
        CursorDraw::ClientSurface { hotspot } => match book.cursor() {
            Some(surface) if view.area.contains(chrome.cursor_at) => {
                let corner = minus(chrome.cursor_at, hotspot);
                render_elements_from_surface_tree(
                    renderer,
                    surface,
                    on_output(sub_of(corner), view),
                    view.scale.as_f64(),
                    1.0,
                    Kind::Cursor,
                )
            }
            _ => Vec::new(),
        },
    }
}

/// A named cursor with its hotspot at `at`, when its canvas touches `view`.
fn named<R: Compositor>(
    shape: CursorShape,
    at: LogicalPoint,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Option<MemoryRenderBufferRenderElement<R>> {
    let corner = minus(at, raster::hotspot(shape));
    let canvas = LogicalSize {
        w: raster::CANVAS,
        h: raster::CANVAS,
    };
    view.area.intersection(&LogicalRect::new(corner, canvas))?;
    let (buffer, side) = buffer_for(book, shape, view.scale);
    let side = f64::from(side);
    let logical = i32::try_from(raster::CANVAS).unwrap_or(i32::MAX);
    MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        on_output(sub_of(corner), view).to_f64(),
        &buffer,
        None,
        Some(Rectangle::<f64, Logical>::from_size((side, side).into())),
        Some(Size::<i32, Logical>::from((logical, logical))),
        Kind::Cursor,
    )
    .ok()
}

/// The buffer for `shape` at `scale` (cached in `book`), and its side in pixels.
fn buffer_for(book: &TextureBook, shape: CursorShape, scale: Scale) -> (MemoryRenderBuffer, u32) {
    let buffer = book.cache().raster(shape, scale.numerator(), || {
        let made = raster::raster(shape, scale);
        let side = i32::try_from(made.side).unwrap_or(i32::MAX);
        MemoryRenderBuffer::from_slice(
            &made.argb,
            Fourcc::Argb8888,
            (side, side),
            1,
            Transform::Normal,
            None,
        )
    });
    let side = casement_core::geometry::buffer_size(
        LogicalSize {
            w: raster::CANVAS,
            h: raster::CANVAS,
        },
        scale,
    )
    .w
    .max(1);
    (buffer, side)
}

/// `at` moved back by `by`.
fn minus(at: LogicalPoint, by: LogicalPoint) -> LogicalPoint {
    LogicalPoint {
        x: at.x.saturating_sub(by.x),
        y: at.y.saturating_sub(by.y),
    }
}
