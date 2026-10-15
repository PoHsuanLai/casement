//! The last image of a window that has closed. The client's surfaces leave the surface book the
//! moment it unmaps, so the loop draws the window once, alone, just before (the same pass window
//! capture uses) and keeps the pixels here. They are baked with the window's rounded bottom
//! corners and drawn as one memory element inside the closing frame, so body and frame fade and
//! shrink as one. There are two kinds of image ([`Pixels`]): a GPU texture the display GPU drew
//! at close, which never leaves the GPU, and a memory buffer made from a read-back, which works
//! on every renderer (GLES, pixman, the dummy one) at the price of one read-back when the window
//! closes and one upload on its first frame. The texture is the display GPU's own, so it is
//! drawn only by that renderer; dropping the body frees it.

use super::{CasementElement, ClipShape, alpha};
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::scene::convert::{on_output, sub_of};
use casement_core::geometry::{LogicalRect, PhysicalRect, PhysicalSize};
use ds_core::geometry::scale::Scale;
use ds_core::vocab::Fraction;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::memory::{
    MemoryRenderBuffer, MemoryRenderBufferRenderElement,
};
use smithay::backend::renderer::element::texture::TextureRenderElement;
use smithay::backend::renderer::element::{Id, Kind};
use smithay::backend::renderer::gles::GlesTexture;
use smithay::utils::{Logical, Rectangle, Size, Transform};
use std::any::Any;

/// Where a closing window's image lives.
#[derive(Debug, Clone)]
pub enum Pixels {
    /// On the CPU, as smithay uploads it (every renderer draws it).
    Memory(MemoryRenderBuffer),
    /// On the display GPU, never read back. Only that GPU's renderer draws it.
    Gpu {
        /// The texture the GPU drew at close.
        texture: GlesTexture,
        /// The element id, the same on every frame so the damage tracker sees one element.
        element: Id,
    },
}

/// A closed window's last image, with its bottom corners already rounded.
#[derive(Debug, Clone)]
pub struct ClosingBody {
    /// Which image this is: two bodies with the same id are the same pixels.
    id: u64,
    /// The image's size in physical pixels.
    size: PhysicalSize,
    /// The pixels.
    pixels: Pixels,
}

impl PartialEq for ClosingBody {
    fn eq(&self, other: &ClosingBody) -> bool {
        (self.id, self.size) == (other.id, other.size)
    }
}

impl Eq for ClosingBody {}

impl ClosingBody {
    /// The body made of `rgba` (`size.w x size.h` pixels of R, G, B, A, premultiplied, rows top
    /// to bottom: what `render_offscreen` reads back), its bottom corners rounded by `radius`
    /// logical pixels at `scale`. `None` for an empty image or bytes that are not that size.
    ///
    /// ```
    /// use casement_core::geometry::PhysicalSize;
    /// use casement_render::ClosingBody;
    /// use ds_core::geometry::scale::Scale;
    ///
    /// let size = PhysicalSize { w: 2, h: 2 };
    /// let body = ClosingBody::from_rgba(1, size, &[255; 16], (0, Scale(120)));
    /// assert_eq!(body.map(|body| body.bytes()), Some(16));
    /// assert!(ClosingBody::from_rgba(1, size, &[0; 3], (0, Scale(120))).is_none());
    /// ```
    pub fn from_rgba(
        id: u64,
        size: PhysicalSize,
        rgba: &[u8],
        (radius, scale): (u32, Scale),
    ) -> Option<ClosingBody> {
        let area = PhysicalRect {
            x: 0,
            y: 0,
            w: size.w,
            h: size.h,
        };
        let shape = ClipShape::new(area, radius, scale);
        let argb = bake(rgba, size, shape)?;
        let side = |n: u32| i32::try_from(n).ok();
        let buffer = MemoryRenderBuffer::from_slice(
            &argb,
            Fourcc::Argb8888,
            (side(size.w)?, side(size.h)?),
            1,
            Transform::Normal,
            None,
        );
        Some(ClosingBody {
            id,
            size,
            pixels: Pixels::Memory(buffer),
        })
    }

    /// The body made of `texture`, which the display GPU drew at `size` with the bottom corners
    /// already cut (`pass::render_window_texture`).
    pub fn from_texture(id: u64, size: PhysicalSize, texture: GlesTexture) -> ClosingBody {
        let pixels = Pixels::Gpu {
            texture,
            element: Id::new(),
        };
        ClosingBody { id, size, pixels }
    }

    /// Where the image lives.
    pub fn pixels(&self) -> &Pixels {
        &self.pixels
    }

    /// What the pixels weigh, in bytes (the loop caps the total it keeps).
    pub fn bytes(&self) -> usize {
        let side = |n: u32| usize::try_from(n).unwrap_or(usize::MAX);
        side(self.size.w)
            .saturating_mul(side(self.size.h))
            .saturating_mul(4)
    }

    /// The body drawn over `content` (global logical pixels: where the closing window's content
    /// is as it shrinks) at `opacity`, stretched to it. `None` when the renderer refuses the
    /// upload, or when the image is a GPU texture of another renderer than `renderer`.
    pub fn element<R: Compositor>(
        &self,
        content: LogicalRect,
        opacity: Fraction,
        view: &OutputView,
        renderer: &mut R,
    ) -> Option<CasementElement<R>> {
        let side = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
        let whole = Rectangle::<f64, Logical>::from_size(
            (f64::from(self.size.w), f64::from(self.size.h)).into(),
        );
        let at = on_output(sub_of(content.origin), view).to_f64();
        let drawn = Size::<i32, Logical>::from((side(content.size.w), side(content.size.h)));
        match &self.pixels {
            Pixels::Memory(buffer) => MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                at,
                buffer,
                Some(alpha(opacity)),
                Some(whole),
                Some(drawn),
                Kind::Unspecified,
            )
            .ok()
            .map(CasementElement::Memory),
            Pixels::Gpu { texture, element } => {
                let any: &dyn Any = texture;
                let texture = any.downcast_ref::<R::TextureId>()?.clone();
                Some(CasementElement::Texture(
                    TextureRenderElement::from_static_texture(
                        element.clone(),
                        renderer.context_id(),
                        at,
                        texture,
                        1,
                        Transform::Normal,
                        Some(alpha(opacity)),
                        Some(whole),
                        Some(drawn),
                        None,
                        Kind::Unspecified,
                    ),
                ))
            }
        }
    }
}

/// `rgba` as Argb8888 bytes (B, G, R, A), every pixel the shape leaves out of its corners
/// weighted by the shape's coverage (premultiplied, so all four channels scale). `None` when
/// `rgba` is not `size` pixels or `size` is empty.
fn bake(rgba: &[u8], size: PhysicalSize, shape: ClipShape) -> Option<Vec<u8>> {
    let width = usize::try_from(size.w).ok().filter(|w| *w > 0)?;
    let height = usize::try_from(size.h).ok().filter(|h| *h > 0)?;
    if rgba.len() != width.checked_mul(height)?.checked_mul(4)? {
        return None;
    }
    let reach = usize::try_from(shape.radius()).unwrap_or(0) + 1;
    let mut out = Vec::with_capacity(rgba.len());
    let (pixels, _) = rgba.as_chunks::<4>();
    for (at, pixel) in pixels.iter().enumerate() {
        let (x, y) = (at % width, at / width);
        let in_corner = y + reach >= height && (x < reach || x + reach >= width);
        let cover = match in_corner {
            true => shape.coverage(x as f32 + 0.5, y as f32 + 0.5),
            false => 1.0,
        };
        let weigh = |channel: u8| (f32::from(channel) * cover).round() as u8;
        out.extend_from_slice(&[
            weigh(pixel[2]),
            weigh(pixel[1]),
            weigh(pixel[0]),
            weigh(pixel[3]),
        ]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(w: u32, h: u32, radius: u32) -> ClipShape {
        let area = PhysicalRect { x: 0, y: 0, w, h };
        ClipShape::new(area, radius, Scale(120))
    }

    fn pixel(bytes: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
        let at = (y * width + x) * 4;
        [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]
    }

    #[test]
    fn channels_swap_to_argb_and_the_corners_are_cut() {
        let (w, h) = (40_u32, 30_u32);
        let rgba: Vec<u8> = (0..w * h).flat_map(|_| [10, 20, 30, 255]).collect();
        let size = PhysicalSize { w, h };
        let got = bake(&rgba, size, shape(w, h, 10)).expect("baked");
        let at = |x, y| pixel(&got, 40, x, y);
        // name, pixel, expected (B, G, R, A)
        let table = [
            ("middle", (20, 15), [30, 20, 10, 255]),
            ("top left stays square", (0, 0), [30, 20, 10, 255]),
            ("top right stays square", (39, 0), [30, 20, 10, 255]),
            ("bottom edge, mid", (20, 29), [30, 20, 10, 255]),
            ("bottom left corner pixel", (0, 29), [0, 0, 0, 0]),
            ("bottom right corner pixel", (39, 29), [0, 0, 0, 0]),
        ];
        for (name, (x, y), want) in table {
            assert_eq!(at(x, y), want, "{name}");
        }
    }

    #[test]
    fn a_square_window_is_only_reordered() {
        let rgba = [1, 2, 3, 4, 5, 6, 7, 8];
        let size = PhysicalSize { w: 2, h: 1 };
        assert_eq!(
            bake(&rgba, size, shape(2, 1, 0)),
            Some(vec![3, 2, 1, 4, 7, 6, 5, 8])
        );
    }

    #[test]
    fn bytes_of_the_wrong_length_or_an_empty_image_make_nothing() {
        let size = PhysicalSize { w: 2, h: 2 };
        assert_eq!(bake(&[0; 15], size, shape(2, 2, 0)), None);
        assert_eq!(bake(&[], PhysicalSize { w: 0, h: 0 }, shape(0, 0, 0)), None);
    }

    #[test]
    fn bodies_are_equal_by_id_and_size() {
        let size = PhysicalSize { w: 1, h: 1 };
        let make = |id, byte| ClosingBody::from_rgba(id, size, &[byte; 4], (0, Scale(120)));
        assert_eq!(make(1, 0), make(1, 9));
        assert_ne!(make(1, 0), make(2, 0));
    }
}
