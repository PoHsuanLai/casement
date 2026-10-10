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
use smithay::backend::renderer::ExportMem;

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

/// Reads `request.area` of `target`, which the last render drew into.
pub fn capture_output<R: Compositor + ExportMem>(
    renderer: &mut R,
    target: &R::Framebuffer<'_>,
    request: &CaptureRequest,
) -> Result<CapturedFrame, RenderError> {
    let _ = (renderer, target, request);
    todo!("P1-C1: capture_output")
}
