//! Writing a captured frame into a client's `wl_shm` buffer.
//!
//! smithay lends a pool as a raw pointer (the client may map it too), so this is the one
//! place outside `gl_raw` that turns a pointer into a slice. Everything else here is safe
//! and tested on plain byte slices.
#![allow(unsafe_code)]

use super::CapturedFrame;
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::reexports::wayland_server::protocol::wl_shm::Format;
use smithay::wayland::shm::{BufferAccessError, BufferData, with_buffer_contents_mut};

/// Why a frame could not be written into a client's buffer.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ShmWriteError {
    /// The buffer is not a shared-memory buffer, or its pool could not be mapped.
    #[error("the buffer cannot be written: {0}")]
    Access(#[from] BufferAccessError),
    /// The buffer's format is not one a capture produces.
    #[error("the buffer's format {0:?} is not argb8888 or xrgb8888")]
    Format(Format),
    /// The buffer's layout is negative, or lies outside its pool.
    #[error("the buffer lies outside its pool")]
    Layout,
    /// The buffer is smaller than the frame.
    #[error("the buffer is smaller than the frame")]
    TooSmall,
}

/// Where a buffer sits in its pool, in bytes and pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Layout {
    offset: usize,
    stride: usize,
    width: usize,
    height: usize,
}

impl Layout {
    /// The layout `data` describes, or `Layout` if a number is negative.
    fn of(data: &BufferData) -> Result<Layout, ShmWriteError> {
        let number = |n: i32| usize::try_from(n).map_err(|_| ShmWriteError::Layout);
        Ok(Layout {
            offset: number(data.offset)?,
            stride: number(data.stride)?,
            width: number(data.width)?,
            height: number(data.height)?,
        })
    }
}

/// Copies `frame` into the shared-memory `buffer`: the same pixels, row by row, at the
/// buffer's own stride. The buffer may be larger than the frame (the rest is left alone) but
/// not smaller.
///
/// ```no_run
/// use casement_render::pass::{CapturedFrame, write_frame};
///
/// # fn demo(buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer, frame: &CapturedFrame) {
/// let _ = write_frame(buffer, frame);
/// # }
/// ```
pub fn write_frame(buffer: &WlBuffer, frame: &CapturedFrame) -> Result<(), ShmWriteError> {
    with_buffer_contents_mut(buffer, |pointer, length, data| {
        let layout = Layout::of(&data)?;
        let format_fits = matches!(data.format, Format::Argb8888 | Format::Xrgb8888);
        if !format_fits {
            return Err(ShmWriteError::Format(data.format));
        }
        // SAFETY: smithay lends `length` bytes at `pointer` for the length of this call and
        // nothing else in this process touches the pool meanwhile; the client may map it too,
        // which is the nature of shared memory (smithay's own docs say the same).
        let pool = unsafe { std::slice::from_raw_parts_mut(pointer, length) };
        copy_rows(pool, layout, frame)
    })?
}

/// The rows of `frame` into `pool` at `layout`.
fn copy_rows(pool: &mut [u8], layout: Layout, frame: &CapturedFrame) -> Result<(), ShmWriteError> {
    let size = |n: u32| usize::try_from(n).unwrap_or(usize::MAX);
    let (width, height) = (size(frame.area.w), size(frame.area.h));
    if layout.width < width || layout.height < height {
        return Err(ShmWriteError::TooSmall);
    }
    // Both formats are four bytes a pixel in the same order; xrgb ignores the alpha byte.
    let row = width.saturating_mul(4);
    let source_stride = size(frame.stride);
    for y in 0..height {
        let from = y.saturating_mul(source_stride);
        let to = layout
            .offset
            .saturating_add(y.saturating_mul(layout.stride));
        let source = frame
            .bytes
            .get(from..from.saturating_add(row))
            .ok_or(ShmWriteError::TooSmall)?;
        let target = pool
            .get_mut(to..to.saturating_add(row))
            .ok_or(ShmWriteError::Layout)?;
        target.copy_from_slice(source);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::PhysicalRect;

    fn frame(w: u32, h: u32) -> CapturedFrame {
        let area = PhysicalRect { x: 0, y: 0, w, h };
        let rgba: Vec<u8> = (0..w * h * 4)
            .map(|n| u8::try_from(n % 251).unwrap_or(0))
            .collect();
        CapturedFrame::from_rgba(area, &rgba)
    }

    #[test]
    fn rows_land_at_the_buffers_stride_and_offset() {
        let frame = frame(2, 2);
        let layout = Layout {
            offset: 4,
            stride: 12,
            width: 3,
            height: 2,
        };
        let mut pool = vec![0xEE; 40];
        copy_rows(&mut pool, layout, &frame).expect("fits");
        assert_eq!(&pool[..4], [0xEE; 4], "before the offset");
        assert_eq!(&pool[4..12], &frame.bytes[..8], "row 0");
        assert_eq!(&pool[12..16], [0xEE; 4], "the padding is left alone");
        assert_eq!(&pool[16..24], &frame.bytes[8..16], "row 1");
    }

    #[test]
    fn a_smaller_buffer_or_a_short_pool_is_refused() {
        let frame = frame(2, 2);
        let small = Layout {
            offset: 0,
            stride: 4,
            width: 1,
            height: 2,
        };
        assert!(matches!(
            copy_rows(&mut [0; 64], small, &frame),
            Err(ShmWriteError::TooSmall)
        ));
        let fits = Layout {
            offset: 0,
            stride: 8,
            width: 2,
            height: 2,
        };
        assert!(matches!(
            copy_rows(&mut [0; 10], fits, &frame),
            Err(ShmWriteError::Layout)
        ));
    }
}
