//! The cursor as data: how big it is, where its hotspot is and, for the compositor's own
//! shapes, its pixels. What a cursor capture session announces and sends; the element
//! ([`super::cursor_elements`]) draws the same cursor on the screen.

use super::raster;
use crate::scene::{ChromeLayer, CursorDraw, CursorShape};
use crate::texture_book::TextureBook;
use casement_core::geometry::{LogicalPoint, LogicalSize};
use ds_core::geometry::scale::Scale;
use smithay::backend::renderer::utils::RendererSurfaceStateUserData;
use smithay::desktop::utils::bbox_from_surface_tree;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::with_states;

/// The largest a client's cursor surface may be drawn, per side, in logical pixels (CASW-08): a
/// pointer is a few dozen pixels, so a bigger surface is a client drawing something else (a fake
/// dialog above the trusted surfaces) or sizing a screencast's buffers by it. Past it the arrow
/// is drawn and announced instead.
pub const MAX_CLIENT_CURSOR: u32 = 256;

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
    match drawn(chrome.cursor, book) {
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

/// Whether a client cursor of `size` is over [`MAX_CLIENT_CURSOR`] on either side.
pub fn exceeds_cap(size: LogicalSize) -> bool {
    size.w > MAX_CLIENT_CURSOR || size.h > MAX_CLIENT_CURSOR
}

/// What is really drawn for `requested`: a client surface wider or taller than
/// [`MAX_CLIENT_CURSOR`] is the arrow. A surface with no size yet (nothing committed) draws
/// nothing either way, so it is left as it is.
pub fn drawn(requested: CursorDraw, book: &TextureBook) -> CursorDraw {
    let too_big = book
        .cursor()
        .and_then(surface_size)
        .is_some_and(exceeds_cap);
    match (requested, too_big) {
        (CursorDraw::ClientSurface { .. }, true) => CursorDraw::Named(CursorShape::Default),
        (other, _) => other,
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

/// The size of what is drawn for a cursor surface: its whole tree, subsurfaces included
/// (CASW-08), from the renderer state of each. A root with no size yet (nothing committed) has
/// none, whatever hangs off it.
fn surface_size(surface: &WlSurface) -> Option<LogicalSize> {
    root_size(surface)?;
    let tree = bbox_from_surface_tree(surface, (0, 0));
    Some(LogicalSize {
        w: u32::try_from(tree.size.w).ok()?,
        h: u32::try_from(tree.size.h).ok()?,
    })
}

/// The size the root surface alone is shown at, from its renderer state.
fn root_size(surface: &WlSurface) -> Option<LogicalSize> {
    with_states(surface, |states| {
        let data = states.data_map.get::<RendererSurfaceStateUserData>()?;
        let dst = data.lock().ok()?.view()?.dst;
        Some(LogicalSize {
            w: u32::try_from(dst.w).ok()?,
            h: u32::try_from(dst.h).ok()?,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CASW-08: the cap is per side, and a pointer-sized surface is under it.
    #[test]
    fn a_cursor_surface_over_the_cap_on_either_side_is_too_big() {
        let size = |w, h| LogicalSize { w, h };
        let cases = [
            ("a pointer", size(24, 24), false),
            (
                "at the cap",
                size(MAX_CLIENT_CURSOR, MAX_CLIENT_CURSOR),
                false,
            ),
            ("one too wide", size(MAX_CLIENT_CURSOR + 1, 24), true),
            ("one too tall", size(24, MAX_CLIENT_CURSOR + 1), true),
            ("a dialog's size", size(1500, 900), true),
            ("a huge buffer", size(8192, 8192), true),
        ];
        for (name, got, want) in cases {
            assert_eq!(exceeds_cap(got), want, "{name}");
        }
    }

    /// Without a client surface in the book nothing is too big: every request stands.
    #[test]
    fn a_request_stands_while_there_is_no_surface_to_measure() {
        let book = TextureBook::default();
        let client = CursorDraw::ClientSurface {
            hotspot: LogicalPoint { x: 1, y: 1 },
        };
        for request in [
            CursorDraw::Hidden,
            CursorDraw::Named(CursorShape::Text),
            client,
        ] {
            assert_eq!(drawn(request, &book), request);
        }
    }
}
