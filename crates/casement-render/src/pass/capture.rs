//! Reading an output's frame back (P1-Z; filled by P1-C1): what `ext_image_copy_capture` and
//! `casementctl shot` return. The frame read back is the one scanout shows: on pixman it is
//! byte-exact.
//!
//! ```no_run
//! use casement_core::geometry::PhysicalRect;
//! use casement_render::pass::{CaptureRequest, CursorInclusion};
//!
//! let whole = PhysicalRect { x: 0, y: 0, w: 1920, h: 1080 };
//! let request = CaptureRequest::new(whole, CursorInclusion::Exclude);
//! assert_eq!(request.area, whole);
//! ```

use crate::Compositor;
use crate::damage::RenderError;
use casement_core::geometry::PhysicalRect;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::ExportMem;
use smithay::utils::{Buffer, Rectangle};

mod bitmap;
mod crop;
mod dmabuf;
mod shm;

pub use bitmap::{ArgbImage, ReadError};
pub use dmabuf::{read_dmabuf, write_dmabuf};
pub use shm::{ShmWriteError, write_frame};

/// Whether the cursor is in the captured image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CursorInclusion {
    /// The cursor is drawn into the image.
    Include,
    /// It is not.
    Exclude,
}

/// What to read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaptureRequest {
    /// The area of the output, in its physical pixels.
    pub area: PhysicalRect,
    /// Whether the cursor is in it.
    pub cursor: CursorInclusion,
}

impl CaptureRequest {
    /// A request.
    pub fn new(area: PhysicalRect, cursor: CursorInclusion) -> CaptureRequest {
        CaptureRequest { area, cursor }
    }
}

/// The pixel format of a captured frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CaptureFormat {
    /// 8 bits per channel with alpha, little-endian `AR24`.
    Argb8888,
    /// 8 bits per channel, alpha ignored, `XR24`.
    Xrgb8888,
}

/// A frame read back from an output.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CapturedFrame {
    /// The area it covers.
    pub area: PhysicalRect,
    /// Its format.
    pub format: CaptureFormat,
    /// Bytes per row.
    pub stride: u32,
    /// The pixels, row after row.
    pub bytes: Vec<u8>,
}

impl CapturedFrame {
    /// A frame of `area` from `rgba`, tightly packed rows of R, G, B, A bytes (what the
    /// renderers read back and what the headless backend keeps as its last frame): the same
    /// pixels as little-endian `Argb8888`, so the capture is the scanout frame byte for byte.
    ///
    /// ```
    /// use casement_core::geometry::PhysicalRect;
    /// use casement_render::pass::{CaptureFormat, CapturedFrame};
    ///
    /// let area = PhysicalRect { x: 0, y: 0, w: 1, h: 1 };
    /// let frame = CapturedFrame::from_rgba(area, &[1, 2, 3, 4]);
    /// assert_eq!(frame.format, CaptureFormat::Argb8888);
    /// assert_eq!(frame.bytes, [3, 2, 1, 4]);
    /// ```
    pub fn from_rgba(area: PhysicalRect, rgba: &[u8]) -> CapturedFrame {
        let bytes = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|&[r, g, b, a]| [b, g, r, a])
            .collect();
        CapturedFrame {
            area,
            format: CaptureFormat::Argb8888,
            stride: area.w.saturating_mul(4),
            bytes,
        }
    }

    /// A frame of `area` from `argb`, tightly packed rows already in little-endian `Argb8888`
    /// byte order (B, G, R, A): a cursor raster as it is.
    ///
    /// ```
    /// use casement_core::geometry::PhysicalRect;
    /// use casement_render::pass::{CaptureFormat, CapturedFrame};
    ///
    /// let area = PhysicalRect { x: 0, y: 0, w: 1, h: 1 };
    /// let frame = CapturedFrame::from_argb(area, vec![3, 2, 1, 4]);
    /// assert_eq!(frame.format, CaptureFormat::Argb8888);
    /// assert_eq!(frame.bytes, [3, 2, 1, 4]);
    /// ```
    pub fn from_argb(area: PhysicalRect, argb: Vec<u8>) -> CapturedFrame {
        CapturedFrame {
            area,
            format: CaptureFormat::Argb8888,
            stride: area.w.saturating_mul(4),
            bytes: argb,
        }
    }
}

/// Reads `request.area` of `target`, which the last render drew into.
pub fn capture_output<R: Compositor + ExportMem>(
    renderer: &mut R,
    target: &R::Framebuffer<'_>,
    request: &CaptureRequest,
) -> Result<CapturedFrame, RenderError> {
    let area = request.area;
    let side = |n: u32| i32::try_from(n).map_err(|_| RenderError::NoTarget);
    let region = Rectangle::<i32, Buffer>::new(
        (area.x, area.y).into(),
        (side(area.w)?, side(area.h)?).into(),
    );
    let fault = |error: R::Error| RenderError::Renderer(error.to_string());
    let mapping = renderer
        .copy_framebuffer(target, region, Fourcc::Abgr8888)
        .map_err(fault)?;
    let rgba = renderer.map_texture(&mapping).map_err(fault)?;
    Ok(CapturedFrame::from_rgba(area, rgba))
}
