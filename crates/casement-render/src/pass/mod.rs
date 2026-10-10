//! Render passes that are not elements. P1-Z: output capture; P3-Z adds the Kawase blur, P4-Z
//! the mask pass.

mod capture;

pub use capture::{CaptureFormat, CaptureRequest, CapturedFrame, CursorInclusion, capture_output};
