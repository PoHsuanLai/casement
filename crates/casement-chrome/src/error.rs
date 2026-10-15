//! Why a title was not drawn.

/// Why [`TitleRaster::render`](crate::TitleRaster::render) made no pixels. None of these is a
/// fault: the frame is drawn without a title.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TitleError {
    /// The title has no visible text.
    #[error("the title is empty")]
    Empty,
    /// The titlebar is empty, or the lights and the edge leave no room between them.
    #[error("the titlebar has no room for a title")]
    NoRoom,
    /// The titlebar is wider or taller than a raster can be.
    #[error("the titlebar is {width} by {height} pixels, too large to draw a title in")]
    TooLarge {
        /// Its width in device pixels.
        width: u32,
        /// Its height in device pixels.
        height: u32,
    },
}
