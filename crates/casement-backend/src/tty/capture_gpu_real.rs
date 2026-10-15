//! The real capture GPU (feature `tty`): a display device's GLES renderer, the one that draws
//! its scanout, lent to draw capture frames into a client's dma-buf with the GPU's own formats
//! and modifiers, and to keep the watched outputs' frames on the GPU (`capture_scene_real.rs`).

use super::capture_gpu::{CaptureGpu, Picture, SceneSupport, renderable_formats};
use super::capture_scene_real::SceneStore;
use crate::backend::BackendError;
use crate::headless::FrameSource;
use casement_core::OutputName;
use casement_core::geometry::{PhysicalRect, PhysicalSize};
use casement_render::RenderError;
use casement_render::pass::{CapturedFrame, CursorInclusion, render_window_dmabuf, write_dmabuf};
use ds_core::geometry::scale::Scale;
use smithay::backend::allocator::Format;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::renderer::gles::GlesRenderer;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;

/// A device's renderer and scene images, shared with its scanouts and its later lenders.
#[derive(Debug)]
pub(super) struct RealCaptureGpu {
    renderer: Rc<RefCell<GlesRenderer>>,
    scenes: Rc<RefCell<SceneStore>>,
    node: Option<u64>,
}

impl RealCaptureGpu {
    /// Lends `renderer`, which draws on the render node numbered `node`, with the device's
    /// `scenes`.
    pub(super) fn new(
        renderer: Rc<RefCell<GlesRenderer>>,
        scenes: Rc<RefCell<SceneStore>>,
        node: Option<u64>,
    ) -> RealCaptureGpu {
        RealCaptureGpu {
            renderer,
            scenes,
            node,
        }
    }

    /// The renderer, or an error while a frame is being drawn with it.
    fn renderer(&self) -> Result<RefMut<'_, GlesRenderer>, BackendError> {
        self.renderer.try_borrow_mut().map_err(|_| busy())
    }

    /// The scene images, or an error while they are in use.
    fn store(&self) -> Result<RefMut<'_, SceneStore>, BackendError> {
        self.scenes.try_borrow_mut().map_err(|_| busy())
    }
}

/// The error of a renderer or store that is in use.
fn busy() -> BackendError {
    BackendError::from(RenderError::Renderer(
        "the display renderer is busy".to_owned(),
    ))
}

impl CaptureGpu for RealCaptureGpu {
    fn formats(&self) -> Vec<Format> {
        // Busy means a frame is being drawn with it: offer nothing now.
        self.renderer
            .try_borrow()
            .map(|renderer| renderable_formats(&*renderer))
            .unwrap_or_default()
    }

    fn write_dmabuf(
        &mut self,
        target: &mut Dmabuf,
        frame: &CapturedFrame,
    ) -> Result<(), BackendError> {
        let mut renderer = self.renderer()?;
        write_dmabuf(&mut *renderer, target, frame).map_err(BackendError::from)
    }

    fn device(&self) -> Option<u64> {
        self.node
    }

    fn scenes(&self) -> SceneSupport {
        SceneSupport::Drawn
    }

    fn draw_scene(
        &mut self,
        output: &OutputName,
        picture: &Picture,
        source: &dyn FrameSource,
    ) -> Result<Vec<PhysicalRect>, BackendError> {
        let mut renderer = self.renderer()?;
        self.store()?.draw(&mut renderer, output, picture, source)
    }

    fn blit_scene(
        &mut self,
        output: &OutputName,
        cursor: CursorInclusion,
        from: PhysicalRect,
        target: &mut Dmabuf,
    ) -> Result<(), BackendError> {
        let mut renderer = self.renderer()?;
        self.store()?
            .blit(&mut renderer, output, cursor, from, target)
    }

    fn read_scene(
        &mut self,
        output: &OutputName,
        cursor: CursorInclusion,
    ) -> Result<CapturedFrame, BackendError> {
        let mut renderer = self.renderer()?;
        self.store()?.read(&mut renderer, output, cursor)
    }

    fn forget_scene(&mut self, output: &OutputName) {
        if let Ok(mut scenes) = self.store() {
            scenes.forget(output);
        }
    }

    fn render_window(
        &mut self,
        target: &mut Dmabuf,
        size: PhysicalSize,
        scale: Scale,
        source: &dyn FrameSource,
    ) -> Result<(), BackendError> {
        let mut renderer = self.renderer()?;
        let elements = source.gles(&mut renderer);
        render_window_dmabuf(&mut *renderer, target, &elements, size, scale)
            .map_err(BackendError::from)
    }
}
