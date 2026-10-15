//! A rasterised title.

use std::sync::Arc;

/// A title's pixels: premultiplied RGBA8, rows top to bottom, as big as the titlebar it was made
/// for at the output's native scale (transparent where there is no ink), so it is laid over the
/// titlebar's own raster at the same place.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TitlePixels {
    /// Which raster this is: unique among a [`TitleRaster`](crate::TitleRaster)'s, so a caller
    /// can tell a new title from the one it drew last frame without comparing bytes.
    pub id: u64,
    /// Width in device pixels.
    pub width: u32,
    /// Height in device pixels.
    pub height: u32,
    /// `width * height * 4` bytes: R, G, B, A, shared so a copy of the pixels is cheap.
    pub rgba: Arc<[u8]>,
}
