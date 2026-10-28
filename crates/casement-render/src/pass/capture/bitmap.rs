//! A small image read back from a client's buffer, and fitted to a capture: the pixels of a
//! cursor a client drew itself (`CursorDraw::ClientSurface`), for a cursor capture session.
//!
//! Shared-memory buffers are read directly ([`ArgbImage::from_shm`]); a dma-buf goes through the
//! renderer ([`super::read_dmabuf`]). The image is then cut to the size the session announced
//! (nearest neighbour: a cursor is a few dozen pixels) and turned with the output
//! ([`ArgbImage::fitted`]), the way the frame is.

use super::shm::Layout;
use crate::gl_raw::with_shm_pool;
use casement_core::geometry::{PhysicalSize, Transform, turn_point, turn_size};
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::reexports::wayland_server::protocol::wl_shm::Format;
use smithay::wayland::shm::BufferAccessError;

/// Why a client's cursor buffer could not be read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ReadError {
    /// The buffer is not a shared-memory buffer, or its pool could not be mapped.
    #[error("the buffer cannot be read: {0}")]
    Access(#[from] BufferAccessError),
    /// The buffer's format is not one a cursor is read from.
    #[error("the buffer's format {0:?} is not argb8888 or xrgb8888")]
    Format(Format),
    /// The buffer's layout is negative, or lies outside its pool.
    #[error("the buffer lies outside its pool")]
    Layout,
}

/// An image of little-endian premultiplied `Argb8888` pixels (B, G, R, A bytes), top row first,
/// tightly packed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ArgbImage {
    /// Its size in pixels.
    pub size: PhysicalSize,
    /// Its pixels: `size.w * size.h * 4` bytes.
    pub argb: Vec<u8>,
}

impl ArgbImage {
    /// An image of `size` from `argb`, which must be `size.w * size.h * 4` bytes (a shorter one
    /// is padded with transparent pixels, a longer one cut).
    ///
    /// ```
    /// use casement_core::geometry::PhysicalSize;
    /// use casement_render::pass::ArgbImage;
    ///
    /// let image = ArgbImage::new(PhysicalSize { w: 1, h: 1 }, vec![1, 2, 3, 4]);
    /// assert_eq!(image.argb, [1, 2, 3, 4]);
    /// ```
    pub fn new(size: PhysicalSize, mut argb: Vec<u8>) -> ArgbImage {
        let (w, h) = sides(size);
        argb.resize(w.saturating_mul(h).saturating_mul(4), 0);
        ArgbImage { size, argb }
    }

    /// An image of `size` from `rgba`, rows of R, G, B, A bytes (what a renderer reads back).
    ///
    /// ```
    /// use casement_core::geometry::PhysicalSize;
    /// use casement_render::pass::ArgbImage;
    ///
    /// let image = ArgbImage::from_rgba(PhysicalSize { w: 1, h: 1 }, &[1, 2, 3, 4]);
    /// assert_eq!(image.argb, [3, 2, 1, 4]);
    /// ```
    pub fn from_rgba(size: PhysicalSize, rgba: &[u8]) -> ArgbImage {
        let argb = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|&[r, g, b, a]| [b, g, r, a])
            .collect();
        ArgbImage::new(size, argb)
    }

    /// The pixels of a shared-memory `buffer` of `argb8888` or `xrgb8888`.
    ///
    /// ```no_run
    /// use casement_render::pass::ArgbImage;
    ///
    /// # fn demo(buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer) {
    /// let _ = ArgbImage::from_shm(buffer);
    /// # }
    /// ```
    pub fn from_shm(buffer: &WlBuffer) -> Result<ArgbImage, ReadError> {
        with_shm_pool(buffer, |pool, data| {
            // `xrgb8888`: the alpha byte is unused, the pixel is opaque.
            let opaque = match data.format {
                Format::Argb8888 => false,
                Format::Xrgb8888 => true,
                other => return Err(ReadError::Format(other)),
            };
            let layout = Layout::of(&data).map_err(|_| ReadError::Layout)?;
            unpack(pool, layout, opaque)
        })?
    }

    /// The image cut to `target` (nearest neighbour; the same image when the size already
    /// fits) and then turned by `transform`, the way the output turns its frame: the result is
    /// `turn_size(target, transform)` large.
    ///
    /// ```
    /// use casement_core::geometry::{PhysicalSize, Transform};
    /// use casement_render::pass::ArgbImage;
    ///
    /// let one = ArgbImage::new(PhysicalSize { w: 1, h: 1 }, vec![9, 8, 7, 255]);
    /// let fitted = one.fitted(PhysicalSize { w: 2, h: 1 }, Transform::R90);
    /// assert_eq!(fitted.size, PhysicalSize { w: 1, h: 2 });
    /// assert_eq!(fitted.argb, [9, 8, 7, 255, 9, 8, 7, 255]);
    /// ```
    pub fn fitted(&self, target: PhysicalSize, transform: Transform) -> ArgbImage {
        self.resampled(target).turned(transform)
    }

    /// The image at `target` size, each pixel taken from the source pixel under its centre.
    fn resampled(&self, target: PhysicalSize) -> ArgbImage {
        let ((sw, sh), (dw, dh)) = (sides(self.size), sides(target));
        if self.size == target {
            return self.clone();
        }
        let mut argb = vec![0_u8; dw.saturating_mul(dh).saturating_mul(4)];
        if sw == 0 || sh == 0 {
            return ArgbImage { size: target, argb };
        }
        let source = |d: usize, from: usize, to: usize| {
            (d.saturating_mul(2).saturating_add(1).saturating_mul(from)
                / to.saturating_mul(2).max(1))
            .min(from.saturating_sub(1))
        };
        for y in 0..dh {
            let row = source(y, sh, dh);
            for x in 0..dw {
                let from = (row * sw + source(x, sw, dw)) * 4;
                let to = (y * dw + x) * 4;
                if let (Some(pixel), Some(slot)) =
                    (self.argb.get(from..from + 4), argb.get_mut(to..to + 4))
                {
                    slot.copy_from_slice(pixel);
                }
            }
        }
        ArgbImage { size: target, argb }
    }

    /// The image turned by `transform` (the frame's rule: [`turn_point`] of each pixel's index).
    pub(super) fn turned(&self, transform: Transform) -> ArgbImage {
        if transform == Transform::Normal {
            return self.clone();
        }
        let size = turn_size(self.size, transform);
        let ((w, h), (tw, _)) = (sides(self.size), sides(size));
        let mut argb = vec![0_u8; self.argb.len()];
        let last = (
            i32::try_from(w).unwrap_or(0).saturating_sub(1),
            i32::try_from(h).unwrap_or(0).saturating_sub(1),
        );
        for y in 0..h {
            for x in 0..w {
                let at = (i32::try_from(x).unwrap_or(0), i32::try_from(y).unwrap_or(0));
                let (tx, ty) = turn_point(transform, at, last);
                let to =
                    (usize::try_from(ty).unwrap_or(0) * tw + usize::try_from(tx).unwrap_or(0)) * 4;
                let from = (y * w + x) * 4;
                if let (Some(pixel), Some(slot)) =
                    (self.argb.get(from..from + 4), argb.get_mut(to..to + 4))
                {
                    slot.copy_from_slice(pixel);
                }
            }
        }
        ArgbImage { size, argb }
    }
}

/// A size as `usize` sides.
pub(super) fn sides(size: PhysicalSize) -> (usize, usize) {
    (
        usize::try_from(size.w).unwrap_or(0),
        usize::try_from(size.h).unwrap_or(0),
    )
}

/// The rows of `pool` at `layout`, packed tight; `opaque` forces every alpha byte to 255 (the
/// source's is unused).
fn unpack(pool: &[u8], layout: Layout, opaque: bool) -> Result<ArgbImage, ReadError> {
    let row = layout.width.saturating_mul(4);
    let mut argb = Vec::with_capacity(row.saturating_mul(layout.height));
    for y in 0..layout.height {
        let from = layout
            .offset
            .saturating_add(y.saturating_mul(layout.stride));
        let line = pool
            .get(from..from.saturating_add(row))
            .ok_or(ReadError::Layout)?;
        argb.extend_from_slice(line);
    }
    if opaque {
        argb.as_chunks_mut::<4>()
            .0
            .iter_mut()
            .for_each(|pixel| pixel[3] = 255);
    }
    let side = |n: usize| u32::try_from(n).map_err(|_| ReadError::Layout);
    Ok(ArgbImage {
        size: PhysicalSize {
            w: side(layout.width)?,
            h: side(layout.height)?,
        },
        argb,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(w: u32, h: u32) -> PhysicalSize {
        PhysicalSize { w, h }
    }

    /// An image whose pixel `n` is `[n, n, n, 255]`.
    fn counted(w: u32, h: u32) -> ArgbImage {
        let argb = (0..w * h)
            .flat_map(|n| {
                let n = u8::try_from(n).unwrap_or(0);
                [n, n, n, 255]
            })
            .collect();
        ArgbImage::new(size(w, h), argb)
    }

    /// The first byte of each pixel, row after row.
    fn ids(image: &ArgbImage) -> Vec<u8> {
        image.argb.chunks(4).map(|pixel| pixel[0]).collect()
    }

    #[test]
    fn rows_come_out_tight_whatever_the_pools_stride() {
        let layout = Layout {
            offset: 2,
            stride: 12,
            width: 2,
            height: 2,
        };
        let mut pool = vec![0xEE; 40];
        pool[2..10].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        pool[14..22].copy_from_slice(&[9, 10, 11, 12, 13, 14, 15, 16]);
        let image = unpack(&pool, layout, false).expect("fits");
        assert_eq!(image.size, size(2, 2));
        assert_eq!(image.argb, (1..=16).collect::<Vec<u8>>());
    }

    #[test]
    fn an_xrgb_source_is_opaque() {
        let layout = Layout {
            offset: 0,
            stride: 4,
            width: 1,
            height: 1,
        };
        let image = unpack(&[1, 2, 3, 0], layout, true).expect("fits");
        assert_eq!(image.argb, [1, 2, 3, 255]);
    }

    #[test]
    fn a_short_pool_is_refused() {
        let layout = Layout {
            offset: 0,
            stride: 8,
            width: 2,
            height: 2,
        };
        assert!(matches!(
            unpack(&[0; 10], layout, false),
            Err(ReadError::Layout)
        ));
    }

    #[test]
    fn resampling_takes_the_pixel_under_each_centre() {
        // name, source, target, the ids that come out
        type Case = (&'static str, (u32, u32), (u32, u32), Vec<u8>);
        let table: Vec<Case> = vec![
            ("same size", (2, 2), (2, 2), vec![0, 1, 2, 3]),
            (
                "doubled",
                (2, 2),
                (4, 4),
                vec![0, 0, 1, 1, 0, 0, 1, 1, 2, 2, 3, 3, 2, 2, 3, 3],
            ),
            ("halved", (4, 2), (2, 1), vec![5, 7]),
            ("one pixel grown", (1, 1), (2, 1), vec![0, 0]),
        ];
        for (name, (sw, sh), (dw, dh), expected) in table {
            let fitted = counted(sw, sh).fitted(size(dw, dh), Transform::Normal);
            assert_eq!(fitted.size, size(dw, dh), "{name}");
            assert_eq!(ids(&fitted), expected, "{name}");
        }
    }

    #[test]
    fn an_empty_source_gives_a_transparent_image() {
        let empty = ArgbImage::new(size(0, 0), Vec::new());
        let fitted = empty.fitted(size(2, 2), Transform::Normal);
        assert_eq!(fitted.argb, [0; 16]);
    }

    #[test]
    fn turning_moves_each_pixel_as_the_frame_moves() {
        // A B C / D E F, as 0 1 2 / 3 4 5.
        let table = [
            (Transform::Normal, (3, 2), vec![0, 1, 2, 3, 4, 5]),
            (Transform::R90, (2, 3), vec![3, 0, 4, 1, 5, 2]),
            (Transform::R180, (3, 2), vec![5, 4, 3, 2, 1, 0]),
            (Transform::R270, (2, 3), vec![2, 5, 1, 4, 0, 3]),
            (Transform::Flipped, (3, 2), vec![2, 1, 0, 5, 4, 3]),
            (Transform::Flipped90, (2, 3), vec![0, 3, 1, 4, 2, 5]),
            (Transform::Flipped180, (3, 2), vec![3, 4, 5, 0, 1, 2]),
            (Transform::Flipped270, (2, 3), vec![5, 2, 4, 1, 3, 0]),
        ];
        for (transform, (w, h), expected) in table {
            let turned = counted(3, 2).fitted(size(3, 2), transform);
            assert_eq!(turned.size, size(w, h), "{transform:?}");
            assert_eq!(ids(&turned), expected, "{transform:?}");
        }
    }

    #[test]
    fn rgba_becomes_argb_byte_order() {
        let image = ArgbImage::from_rgba(size(2, 1), &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(image.argb, [3, 2, 1, 4, 7, 6, 5, 8]);
    }
}
