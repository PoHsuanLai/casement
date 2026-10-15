//! The display GPU's capture scenes (feature `tty`): while a capture is open on an output, each
//! frame shown is drawn once more into two GLES textures, one with the cursor and one without,
//! that stay on the GPU. A capture frame is then a texture-to-dma-buf draw on the same GPU
//! (`casement_render::pass::blit_texture`) with no CPU read; only a shared-memory client costs a
//! read-back, and only for the frames it asks for.

use super::capture_gpu::Picture;
use crate::backend::BackendError;
use crate::headless::{Cursorless, FrameSource, transform_of};
use casement_core::OutputName;
use casement_core::geometry::{PhysicalRect, PhysicalSize};
use casement_render::pass::{CapturedFrame, CursorInclusion, blit_texture};
use casement_render::render_output;
use smithay::backend::allocator::Fourcc;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::gles::{GlesRenderer, GlesTexture};
use smithay::backend::renderer::{Bind, ExportMem, Offscreen};
use smithay::utils::{Buffer, Rectangle, Size};
use std::collections::BTreeMap;

/// Every watched output's scenes.
#[derive(Debug, Default)]
pub(super) struct SceneStore {
    scenes: BTreeMap<OutputName, Scenes>,
}

/// One output's two images and the picture they were made for.
#[derive(Debug)]
struct Scenes {
    picture: Picture,
    shown: Layer,
    bare: Layer,
}

/// One GPU image of the frame and the tracker that knows what changed in it.
#[derive(Debug)]
struct Layer {
    texture: GlesTexture,
    tracker: OutputDamageTracker,
    /// The buffer age the next frame draws with: 0 before the first frame, then 1.
    age: usize,
}

impl SceneStore {
    /// Draws `source` as the frame `output` shows: a new pair of images when the picture
    /// changed. The area that changed in the frame with the cursor.
    pub(super) fn draw(
        &mut self,
        renderer: &mut GlesRenderer,
        output: &OutputName,
        picture: &Picture,
        source: &dyn FrameSource,
    ) -> Result<Vec<PhysicalRect>, BackendError> {
        let current = self.scenes.get(output).map(|scenes| scenes.picture);
        if current != Some(*picture) {
            self.scenes
                .insert(output.clone(), Scenes::new(renderer, picture)?);
        }
        let scenes = self.scenes.get_mut(output).ok_or(BackendError::NoFrame)?;
        let damage = scenes.shown.draw(renderer, source)?;
        scenes.bare.draw(renderer, &Cursorless(source))?;
        Ok(damage)
    }

    /// Draws `from`, a part of the frame last drawn for `output`, into the client's `target`.
    pub(super) fn blit(
        &self,
        renderer: &mut GlesRenderer,
        output: &OutputName,
        cursor: CursorInclusion,
        from: PhysicalRect,
        target: &mut Dmabuf,
    ) -> Result<(), BackendError> {
        let (_, layer) = self.layer(output, cursor)?;
        blit_texture(renderer, &layer.texture, from, target).map_err(BackendError::from)
    }

    /// The frame last drawn for `output`, read back.
    pub(super) fn read(
        &self,
        renderer: &mut GlesRenderer,
        output: &OutputName,
        cursor: CursorInclusion,
    ) -> Result<CapturedFrame, BackendError> {
        let (size, layer) = self.layer(output, cursor)?;
        let region = Rectangle::<i32, Buffer>::from_size(buffer_size(size)?);
        let mut texture = layer.texture.clone();
        let framebuffer = renderer.bind(&mut texture)?;
        let mapping = renderer.copy_framebuffer(&framebuffer, region, Fourcc::Abgr8888)?;
        let rgba = renderer.map_texture(&mapping)?;
        let area = PhysicalRect {
            x: 0,
            y: 0,
            w: size.w,
            h: size.h,
        };
        Ok(CapturedFrame::from_rgba(area, rgba))
    }

    /// Frees `output`'s images.
    pub(super) fn forget(&mut self, output: &OutputName) {
        self.scenes.remove(output);
    }

    /// The image of `output` that does or does not show the cursor, and its size.
    fn layer(
        &self,
        output: &OutputName,
        cursor: CursorInclusion,
    ) -> Result<(PhysicalSize, &Layer), BackendError> {
        let scenes = self.scenes.get(output).ok_or(BackendError::NoFrame)?;
        let layer = match cursor {
            CursorInclusion::Exclude => &scenes.bare,
            _ => &scenes.shown,
        };
        Ok((scenes.picture.size, layer))
    }
}

impl Scenes {
    fn new(renderer: &mut GlesRenderer, picture: &Picture) -> Result<Scenes, BackendError> {
        Ok(Scenes {
            picture: *picture,
            shown: Layer::new(renderer, picture)?,
            bare: Layer::new(renderer, picture)?,
        })
    }
}

impl Layer {
    fn new(renderer: &mut GlesRenderer, picture: &Picture) -> Result<Layer, BackendError> {
        let size = buffer_size(picture.size)?;
        let texture: GlesTexture = renderer.create_buffer(Fourcc::Abgr8888, size)?;
        let tracker = OutputDamageTracker::new(
            (size.w, size.h),
            picture.scale.as_f64(),
            transform_of(picture.transform),
        );
        Ok(Layer {
            texture,
            tracker,
            age: 0,
        })
    }

    /// Draws `source` into the image; the area that changed.
    fn draw(
        &mut self,
        renderer: &mut GlesRenderer,
        source: &dyn FrameSource,
    ) -> Result<Vec<PhysicalRect>, BackendError> {
        let elements = source.gles(renderer);
        let mut framebuffer = renderer.bind(&mut self.texture)?;
        let rendered = render_output(
            renderer,
            &mut framebuffer,
            &mut self.tracker,
            &elements,
            self.age,
        )?;
        self.age = 1;
        Ok(rendered.damage)
    }
}

/// `size` as smithay's buffer size.
fn buffer_size(size: PhysicalSize) -> Result<Size<i32, Buffer>, BackendError> {
    let fits = |n: u32| i32::try_from(n).ok();
    fits(size.w)
        .zip(fits(size.h))
        .map(|(w, h)| Size::from((w, h)))
        .ok_or(BackendError::OutputTooLarge {
            w: size.w,
            h: size.h,
        })
}
