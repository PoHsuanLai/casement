//! A window title as the renderer takes it: premultiplied RGBA pixels made elsewhere
//! (`casement-chrome`), as big as the titlebar at the output's native scale. The renderer reads
//! no font and names no text crate (boundary rows B4 and B15).

use std::sync::Arc;

/// The title laid over a titlebar: transparent where there is no ink.
#[derive(Debug, Clone)]
pub struct TitleDraw {
    /// Which raster this is. Two draws with the same id are the same pixels; the renderer's
    /// cache and the damage tracker go by it.
    pub id: u64,
    /// Width in device pixels (the titlebar's).
    pub width: u32,
    /// Height in device pixels (the titlebar's).
    pub height: u32,
    /// `width * height * 4` bytes, premultiplied R, G, B, A, rows top to bottom.
    pub rgba: Arc<[u8]>,
}

impl PartialEq for TitleDraw {
    fn eq(&self, other: &TitleDraw) -> bool {
        (self.id, self.width, self.height) == (other.id, other.width, other.height)
    }
}

impl Eq for TitleDraw {}

impl TitleDraw {
    /// The pixels as the Argb8888 buffer the other frame rasters use (little-endian: B, G, R,
    /// A), or nothing when the bytes are not `width * height` pixels.
    ///
    /// ```
    /// use casement_render::TitleDraw;
    /// use std::sync::Arc;
    ///
    /// let draw = TitleDraw { id: 1, width: 1, height: 1, rgba: Arc::from([1u8, 2, 3, 4]) };
    /// assert_eq!(draw.argb(), Some(vec![3, 2, 1, 4]));
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

    fn draw(width: u32, height: u32, bytes: &[u8]) -> TitleDraw {
        TitleDraw {
            id: 7,
            width,
            height,
            rgba: Arc::from(bytes),
        }
    }

    #[test]
    fn the_channels_swap_into_argb_order() {
        let got = draw(2, 1, &[10, 20, 30, 255, 0, 0, 0, 0]).argb();
        assert_eq!(got, Some(vec![30, 20, 10, 255, 0, 0, 0, 0]));
    }

    #[test]
    fn bytes_of_the_wrong_length_make_nothing() {
        assert_eq!(draw(2, 1, &[0; 4]).argb(), None);
        assert_eq!(draw(1, 1, &[0; 8]).argb(), None);
        assert_eq!(draw(0, 0, &[]).argb(), Some(vec![]));
    }

    #[test]
    fn draws_are_equal_by_raster_id_and_size() {
        assert_eq!(draw(1, 1, &[1, 2, 3, 4]), draw(1, 1, &[9, 9, 9, 9]));
        let mut other = draw(1, 1, &[1, 2, 3, 4]);
        other.id = 8;
        assert_ne!(other, draw(1, 1, &[1, 2, 3, 4]));
    }
}
