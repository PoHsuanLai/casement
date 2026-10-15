//! The real capture GPU (feature `tty`): a display device's GLES renderer, the one that draws
//! its scanout, lent to draw capture frames into a client's dma-buf with the GPU's own formats
//! and modifiers.

use super::capture_gpu::{CaptureGpu, renderable_formats};
use crate::backend::BackendError;
use casement_render::RenderError;
use casement_render::pass::{CapturedFrame, write_dmabuf};
use smithay::backend::allocator::Format;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::renderer::gles::GlesRenderer;
use std::cell::RefCell;
use std::rc::Rc;

/// A device's renderer, shared with its scanouts.
#[derive(Debug)]
pub(super) struct RealCaptureGpu {
    renderer: Rc<RefCell<GlesRenderer>>,
}

impl RealCaptureGpu {
    /// Lends `renderer`.
    pub(super) fn new(renderer: Rc<RefCell<GlesRenderer>>) -> RealCaptureGpu {
        RealCaptureGpu { renderer }
    }
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
        let mut renderer = self.renderer.try_borrow_mut().map_err(|_| {
            BackendError::from(RenderError::Renderer(
                "the display renderer is busy".to_owned(),
            ))
        })?;
        write_dmabuf(&mut *renderer, target, frame).map_err(BackendError::from)
    }
}
