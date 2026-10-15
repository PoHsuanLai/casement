//! The cursor as data: how big it is, where its hotspot is and, for the compositor's own
//! shapes, its pixels. What a cursor capture session announces and sends; the element
//! ([`super::cursor_elements`]) draws the same cursor on the screen.

use super::raster;
use crate::scene::{ChromeLayer, CursorDraw};
use crate::texture_book::TextureBook;
use casement_core::geometry::{LogicalPoint, LogicalSize};
use ds_core::geometry::scale::Scale;
use smithay::backend::renderer::utils::RendererSurfaceStateUserData;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::with_states;

/// What the cursor looks like, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CursorFace {
    /// The image's size.
    pub size: LogicalSize,
    /// The hotspot's offset from the image's top-left corner.
    pub hotspot: LogicalPoint,
}

/// The face of the cursor `chrome` draws; none while it is hidden or a client surface that has
/// no size yet.
///
/// ```
/// use casement_core::geometry::LogicalPoint;
/// use casement_render::{ChromeLayer, CursorDraw, CursorShape, TextureBook, cursor_face};
///
/// let chrome = ChromeLayer {
///     cursor: CursorDraw::Named(CursorShape::Default),
///     cursor_at: LogicalPoint { x: 0, y: 0 },
///     covers: Vec::new(),
/// };
/// let face = cursor_face(&chrome, &TextureBook::default()).expect("a named cursor has a face");
/// assert_eq!((face.size.w, face.size.h), (24, 24));
/// ```
pub fn cursor_face(chrome: &ChromeLayer, book: &TextureBook) -> Option<CursorFace> {
    match chrome.cursor {
        CursorDraw::Hidden => None,
        CursorDraw::Named(shape) => Some(CursorFace {
            size: LogicalSize {
                w: raster::CANVAS,
                h: raster::CANVAS,
            },
            hotspot: raster::hotspot(shape),
        }),
        CursorDraw::ClientSurface { hotspot } => {
            let size = surface_size(book.cursor()?)?;
            Some(CursorFace { size, hotspot })
        }
    }
}

/// The pixels of the cursor at `scale`, as little-endian premultiplied `Argb8888` rows (B, G,
/// R, A bytes, top row first), `buffer_size(face.size, scale)` on a side. Only the
/// compositor's own shapes have pixels here: a client's cursor surface is not read back.
///
/// ```
/// use casement_core::geometry::LogicalPoint;
/// use casement_render::{ChromeLayer, CursorDraw, CursorShape, cursor_pixels};
/// use ds_core::geometry::scale::Scale;
///
/// let chrome = ChromeLayer {
///     cursor: CursorDraw::Named(CursorShape::Text),
///     cursor_at: LogicalPoint { x: 0, y: 0 },
///     covers: Vec::new(),
/// };
/// let argb = cursor_pixels(&chrome, Scale(120)).expect("a named cursor has pixels");
/// assert_eq!(argb.len(), 24 * 24 * 4);
/// ```
pub fn cursor_pixels(chrome: &ChromeLayer, scale: Scale) -> Option<Vec<u8>> {
    match chrome.cursor {
        CursorDraw::Named(shape) => Some(raster::raster(shape, scale).argb),
        _ => None,
    }
}

/// The size a surface is shown at, from its renderer state.
fn surface_size(surface: &WlSurface) -> Option<LogicalSize> {
    with_states(surface, |states| {
        let data = states.data_map.get::<RendererSurfaceStateUserData>()?;
        let dst = data.lock().ok()?.view()?.dst;
        Some(LogicalSize {
            w: u32::try_from(dst.w).ok()?,
            h: u32::try_from(dst.h).ok()?,
        })
    })
}
