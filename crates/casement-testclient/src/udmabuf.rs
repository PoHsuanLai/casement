//! The client's half of a dma-buf buffer for the `--dmabuf soft` path (A4): a sealed memfd
//! with the right stride, holding a pattern.
//!
//! Turning a memfd into a dma-buf is the `UDMABUF_CREATE` ioctl on `/dev/udmabuf`, which needs
//! `unsafe` (smithay wraps it; this crate reaches no smithay, B9, and `unsafe` lives in one
//! module only, B18). So the conversion belongs to the test kit: it takes
//! [`UdmabufSource::memfd`] and [`UdmabufSource::layout`], makes the dma-buf with its own
//! allocator (never smithay's `UdmabufAllocator::create_buffer`, which over-allocates 8x), and
//! hands the dma-buf's descriptor to the protocol helper that sends it.

use crate::pattern::Pattern;
use casement_core::geometry::PhysicalSize;
use rustix::fs::{MemfdFlags, SealFlags, fcntl_add_seals, ftruncate, memfd_create};
use std::fs::File;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::fs::FileExt;

/// ARGB8888 is four bytes a pixel.
const BYTES_PER_PIXEL: u32 = 4;
/// `/dev/udmabuf` takes whole pages.
const PAGE: u64 = 4096;

/// The shape of a source buffer, as the dma-buf parameters need it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// Width and height in pixels.
    pub size: PhysicalSize,
    /// Bytes from one row to the next.
    pub stride: u32,
    /// The memfd's length: a whole number of pages.
    pub length: u64,
}

/// A memfd holding a pattern, sealed so `/dev/udmabuf` accepts it.
#[derive(Debug)]
pub struct UdmabufSource {
    memfd: OwnedFd,
    layout: Layout,
}

/// Why a source could not be made.
#[derive(Debug, thiserror::Error)]
pub enum UdmabufError {
    /// A memfd call failed.
    #[error("memfd: {0}")]
    Memfd(#[from] rustix::io::Errno),
    /// Writing the pattern failed.
    #[error("write: {0}")]
    Write(#[from] std::io::Error),
}

impl Layout {
    /// The layout of a `size` ARGB8888 buffer: stride is `width * 4` bytes (not bits), the
    /// length rounds up to a page.
    pub fn of(size: PhysicalSize) -> Layout {
        let stride = size.w * BYTES_PER_PIXEL;
        let bytes = u64::from(stride) * u64::from(size.h);
        Layout {
            size,
            stride,
            length: bytes.div_ceil(PAGE) * PAGE,
        }
    }
}

impl UdmabufSource {
    /// A sealed memfd of `size` holding `pattern`.
    pub fn new(size: PhysicalSize, pattern: Pattern) -> Result<UdmabufSource, UdmabufError> {
        let layout = Layout::of(size);
        let memfd = memfd_create(
            "casement-udmabuf",
            MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
        )?;
        ftruncate(&memfd, layout.length)?;
        let file = File::from(memfd.as_fd().try_clone_to_owned()?);
        file.write_all_at(&pattern.bytes(size), 0)?;
        // udmabuf refuses a memfd that can still shrink.
        fcntl_add_seals(&memfd, SealFlags::SHRINK)?;
        Ok(UdmabufSource { memfd, layout })
    }

    /// The memfd, for the test kit's allocator.
    pub fn memfd(&self) -> &OwnedFd {
        &self.memfd
    }

    /// Its layout.
    pub fn layout(&self) -> Layout {
        self.layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stride_is_bytes_not_bits_and_the_length_is_whole_pages() {
        let layout = Layout::of(PhysicalSize { w: 1920, h: 1080 });
        assert_eq!(layout.stride, 7680);
        assert_eq!(layout.length, 7680 * 1080);
        let odd = Layout::of(PhysicalSize { w: 5, h: 3 });
        assert_eq!((odd.stride, odd.length), (20, 4096));
    }

    #[test]
    fn a_source_is_sealed_against_shrinking_and_holds_its_pattern() {
        let size = PhysicalSize { w: 16, h: 8 };
        let source = UdmabufSource::new(size, Pattern::Solid(0xff11_2233)).expect("source");
        let file = File::from(source.memfd().as_fd().try_clone_to_owned().expect("dup"));
        let mut first = [0u8; 4];
        file.read_exact_at(&mut first, 0).expect("read");
        assert_eq!(u32::from_le_bytes(first), 0xff11_2233);
        assert!(
            ftruncate(source.memfd(), 16).is_err(),
            "the memfd is sealed against shrinking"
        );
    }
}
