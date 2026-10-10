//! Render passes that are not elements. P1-Z: output capture; P3-Z adds the Kawase blur, P4-Z
//! the mask pass.

mod capture;
mod window;

pub use capture::{
    CaptureFormat, CaptureRequest, CapturedFrame, CursorInclusion, ShmWriteError, capture_output,
    write_frame,
};
pub use window::{render_window, window_capture_size, window_tree};
