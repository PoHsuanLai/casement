//! Render passes that are not elements. P1-Z: output capture; P3-Z adds the Kawase blur, P4-Z
//! the mask pass.

mod blur_kawase;
mod capture;
mod thumb;
mod window;
mod window_texture;

pub use blur_kawase::{BlurLook, KawasePlan, level_size};
pub(crate) use blur_kawase::{Finished, capture as capture_blur, finish as finish_blur};
pub use capture::{
    ArgbImage, CaptureFormat, CaptureRequest, CapturedFrame, CursorInclusion, DrawnView, ReadError,
    ShmWriteError, SourceRect, blit_texture, capture_output, crop_pixels, read_dmabuf,
    write_dmabuf, write_frame,
};
pub use thumb::{ThumbLayer, compose_thumb, thumb_dest};
pub use window::{render_window, render_window_dmabuf, window_capture_size, window_tree};
pub use window_texture::render_window_texture;
