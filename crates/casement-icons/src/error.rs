//! Why an icon file gave no pixels.

use std::path::PathBuf;

/// Why [`load`](crate::load) made no pixels. None of these is a fault: the next place to look
/// is tried, and the last resort is the plain plate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IconError {
    /// The file could not be read.
    #[error("cannot read the icon {path}")]
    Read {
        /// The file.
        path: PathBuf,
        /// Why.
        #[source]
        source: std::io::Error,
    },
    /// The file is larger than an icon should be.
    #[error("the icon {path} is too large")]
    TooLarge {
        /// The file.
        path: PathBuf,
    },
    /// The picture could not be decoded.
    #[error("cannot decode the icon: {0}")]
    Decode(String),
    /// The wanted edge is zero, or too large for a picture.
    #[error("no picture of {0} pixels")]
    Edge(u32),
}
