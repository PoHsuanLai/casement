//! A KMS writeback frame, compared with a pixman golden (P1-V). Pure: bytes in, verdict out.
//!
//! A writeback connector hands the frame back as a raw buffer: rows of `pitch` bytes, 4 bytes a
//! pixel, little-endian `XRGB8888` or `ARGB8888` (blue first in memory). [`decode`] turns that
//! into an `RgbaImage`; [`tolerance`] is the golden tier a hardware frame is held to (the
//! hardware blends and rounds differently from pixman, so never `Exact`); [`compare_raw`] does
//! both and the comparison. `dev/vkms/accept.sh` reaches it through the `writeback-compare`
//! binary.

use casement_render::golden::{Bless, Golden, GoldenOutcome, GoldenTier, compare};
use image::{Rgba, RgbaImage};
use std::path::Path;

/// How a writeback buffer lays out a pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PixelFormat {
    /// `XRGB8888`: the fourth byte is padding; every pixel is opaque.
    Xrgb8888,
    /// `ARGB8888`: the fourth byte is alpha.
    Argb8888,
}

impl PixelFormat {
    /// The format named by its DRM fourcc name (`XR24`, `AR24`) or its long name.
    ///
    /// ```
    /// use casement_testkit::writeback::PixelFormat;
    /// assert_eq!(PixelFormat::named("XR24"), Some(PixelFormat::Xrgb8888));
    /// assert_eq!(PixelFormat::named("nope"), None);
    /// ```
    pub fn named(name: &str) -> Option<PixelFormat> {
        match name {
            "XR24" | "XRGB8888" => Some(PixelFormat::Xrgb8888),
            "AR24" | "ARGB8888" => Some(PixelFormat::Argb8888),
            _ => None,
        }
    }
}

/// The shape of a raw writeback buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RawFrame {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Bytes from one row's start to the next (at least `width * 4`).
    pub pitch: u32,
    /// The pixel layout.
    pub format: PixelFormat,
}

impl RawFrame {
    /// A frame of `width` x `height` pixels with `pitch` bytes per row.
    pub fn new(width: u32, height: u32, pitch: u32, format: PixelFormat) -> RawFrame {
        RawFrame {
            width,
            height,
            pitch,
            format,
        }
    }
}

/// Why a raw buffer is not a frame.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum WritebackError {
    /// The pitch cannot hold a row of pixels.
    #[error("pitch {pitch} is smaller than a row of {width} pixels ({needed} bytes)")]
    PitchTooSmall {
        /// The pitch given.
        pitch: u32,
        /// The width given.
        width: u32,
        /// The bytes a row needs.
        needed: u32,
    },
    /// The buffer holds fewer bytes than the shape needs.
    #[error("buffer has {have} bytes, the frame needs {needed}")]
    Short {
        /// Bytes present.
        have: usize,
        /// Bytes needed.
        needed: u64,
    },
    /// A frame of no pixels.
    #[error("the frame is empty")]
    Empty,
}

/// The pixels of `bytes` as `frame` describes them. An `XRGB8888` frame comes out opaque.
///
/// ```
/// use casement_testkit::writeback::{PixelFormat, RawFrame, decode};
/// let frame = RawFrame::new(1, 1, 4, PixelFormat::Xrgb8888);
/// let image = decode(&frame, &[3, 2, 1, 0]).expect("a frame");
/// assert_eq!(image.get_pixel(0, 0).0, [1, 2, 3, 255]);
/// ```
pub fn decode(frame: &RawFrame, bytes: &[u8]) -> Result<RgbaImage, WritebackError> {
    if frame.width == 0 || frame.height == 0 {
        return Err(WritebackError::Empty);
    }
    let row = frame.width.saturating_mul(4);
    if frame.pitch < row {
        return Err(WritebackError::PitchTooSmall {
            pitch: frame.pitch,
            width: frame.width,
            needed: row,
        });
    }
    let needed = u64::from(frame.pitch) * u64::from(frame.height - 1) + u64::from(row);
    if (bytes.len() as u64) < needed {
        return Err(WritebackError::Short {
            have: bytes.len(),
            needed,
        });
    }
    Ok(RgbaImage::from_fn(frame.width, frame.height, |x, y| {
        let at = (y as usize) * (frame.pitch as usize) + (x as usize) * 4;
        let alpha = match frame.format {
            PixelFormat::Xrgb8888 => 255,
            PixelFormat::Argb8888 => bytes[at + 3],
        };
        Rgba([bytes[at + 2], bytes[at + 1], bytes[at], alpha])
    }))
}

/// The golden tier a hardware writeback frame is held to: every channel within `channel` steps
/// on all but `parts_per_million` of the pixels.
///
/// ```
/// use casement_render::golden::GoldenTier;
/// use casement_testkit::writeback::tolerance;
/// assert_eq!(tolerance(2, 1000), GoldenTier::Soft { channel: 2, parts_per_million: 1000 });
/// ```
pub fn tolerance(channel: u8, parts_per_million: u32) -> GoldenTier {
    GoldenTier::Soft {
        channel,
        parts_per_million,
    }
}

/// Decodes `bytes` and compares the frame with the golden at `scene` (path without `.png`)
/// under `tier`. A difference writes the frame and a diff image into `out_dir`.
pub fn compare_raw(
    frame: &RawFrame,
    bytes: &[u8],
    scene: &str,
    tier: GoldenTier,
    out_dir: &Path,
) -> Result<GoldenOutcome, WritebackError> {
    let image = decode(frame, bytes)?;
    let golden = Golden {
        scene: scene.to_owned(),
        tier,
    };
    Ok(compare(&golden, &image, out_dir, Bless::No))
}
