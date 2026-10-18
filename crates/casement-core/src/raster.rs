//! A raster made outside the renderer and laid over a frame: a window title, an icon. Chrome
//! makes it (`casement-chrome`, the only crate that reaches the text stack), the renderer takes
//! it (`casement-render`, which names no text crate), and this is the one type between them.

use std::sync::Arc;

/// Which raster an image is: unique among those one maker has made, so a caller can tell a new
/// image from the one it drew last frame without comparing bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RasterId(pub u64);

/// An image: premultiplied RGBA8, rows top to bottom, in device pixels, transparent where there
/// is no ink. Two images are equal when they have the same id and size: the renderer's cache and
/// the damage tracker go by that.
#[derive(Debug, Clone)]
pub struct RasterImage {
    /// Which image this is. Two with the same id are the same pixels.
    pub id: RasterId,
    /// Width in device pixels.
    pub width: u32,
    /// Height in device pixels.
    pub height: u32,
    /// `width * height * 4` bytes: R, G, B, A, shared so a copy of the pixels is cheap.
    pub rgba: Arc<[u8]>,
}

impl PartialEq for RasterImage {
    fn eq(&self, other: &RasterImage) -> bool {
        (self.id, self.width, self.height) == (other.id, other.width, other.height)
    }
}

impl Eq for RasterImage {}

impl RasterImage {
    /// The pixels as the Argb8888 buffer the other frame rasters use (little-endian: B, G, R,
    /// A), or nothing when the bytes are not `width * height` pixels.
    ///
    /// ```
    /// use casement_core::raster::{RasterId, RasterImage};
    /// use std::sync::Arc;
    ///
    /// let image = RasterImage {
    ///     id: RasterId(1),
    ///     width: 1,
    ///     height: 1,
    ///     rgba: Arc::from([1u8, 2, 3, 4]),
    /// };
    /// assert_eq!(image.argb(), Some(vec![3, 2, 1, 4]));
    /// ```
    pub fn argb(&self) -> Option<Vec<u8>> {
        let wanted = usize::try_from(u64::from(self.width) * u64::from(self.height) * 4).ok()?;
        (self.rgba.len() == wanted).then(|| {
            self.rgba
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|px| [px[2], px[1], px[0], px[3]])
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32, bytes: &[u8]) -> RasterImage {
        RasterImage {
            id: RasterId(7),
            width,
            height,
            rgba: Arc::from(bytes),
        }
    }

    #[test]
    fn the_channels_swap_into_argb_order() {
        let got = image(2, 1, &[10, 20, 30, 255, 0, 0, 0, 0]).argb();
        assert_eq!(got, Some(vec![30, 20, 10, 255, 0, 0, 0, 0]));
    }

    #[test]
    fn bytes_of_the_wrong_length_make_nothing() {
        assert_eq!(image(2, 1, &[0; 4]).argb(), None);
        assert_eq!(image(1, 1, &[0; 8]).argb(), None);
        assert_eq!(image(0, 0, &[]).argb(), Some(vec![]));
    }

    #[test]
    fn images_are_equal_by_raster_id_and_size() {
        assert_eq!(image(1, 1, &[1, 2, 3, 4]), image(1, 1, &[9, 9, 9, 9]));
        let mut other = image(1, 1, &[1, 2, 3, 4]);
        other.id = RasterId(8);
        assert_ne!(other, image(1, 1, &[1, 2, 3, 4]));
    }
}
