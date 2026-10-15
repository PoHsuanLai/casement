//! A window's shadow as buffers smithay can draw: the tiles of `shadow.rs` uploaded once per
//! window (two windows never share a buffer, so no two elements in a frame share an id), and
//! the key the tiles are made under, which holds no window size.

use super::raster::Raster;
use super::shadow::Tiles;
use super::slices::Piece;
use super::to_buffer;
use casement_core::window::{FrameLook, ShadowLayer};
use ds_core::geometry::scale::Scale;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;
use std::rc::Rc;

/// What a shadow's tiles are made from: the scale's numerator, the corner radius, the layers.
/// The size of the window is not in it.
pub(crate) type ShadowKey = (u32, u32, [ShadowLayer; 2]);

/// The key of `look`'s shadow at `scale`.
pub(crate) fn key(look: &FrameLook, scale: Scale) -> ShadowKey {
    (scale.numerator(), look.radius, look.shadow)
}

/// One window's shadow tiles as buffers, with the rasters they came from (for their sizes).
#[derive(Debug, Clone)]
pub(crate) struct ShadowSet {
    tiles: Rc<Tiles>,
    corners: [MemoryRenderBuffer; 4],
    edges: [MemoryRenderBuffer; 4],
}

impl ShadowSet {
    /// The buffers of `tiles`.
    pub(crate) fn new(tiles: Rc<Tiles>) -> ShadowSet {
        ShadowSet {
            corners: tiles.corners.each_ref().map(to_buffer),
            edges: tiles.edges.each_ref().map(to_buffer),
            tiles,
        }
    }

    /// How far the corners reach into the window, in logical pixels.
    pub(crate) fn inner(&self) -> u32 {
        self.tiles.inner
    }

    /// The buffer of `piece` and its raster.
    pub(crate) fn of(&self, piece: Piece) -> (&MemoryRenderBuffer, &Raster) {
        match piece {
            Piece::Corner(c) => (&self.corners[c.index()], &self.tiles.corners[c.index()]),
            Piece::Edge(s) => (&self.edges[s.index()], &self.tiles.edges[s.index()]),
        }
    }
}
