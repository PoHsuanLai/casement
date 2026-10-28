//! Capture on a real seat through the display GPU (wave-2026-10-15/dmabuf): while a capture is
//! open on an output and the display GPU keeps scenes, every frame shown is drawn once more
//! into images that stay on the GPU, and a capture frame is drawn from them into the client's
//! dma-buf (or read back from them for shared memory) when it is asked for. The software
//! shadow (`readback.rs`) draws only when no GPU does: a lender that keeps no scenes, a busy
//! renderer, a draw that failed.

use super::TtyBackend;
use super::capture_gpu::{BodyCopy, CaptureRoute, Picture, SceneSupport, body_copy, capture_route};
use crate::backend::BackendError;
use crate::headless::{FramePixels, FrameSource, PresentedFrame};
use casement_core::OutputName;
use casement_core::geometry::{PhysicalRect, PhysicalSize};
use casement_core::time::{Mono, VblankSeq};
use casement_render::pass::CapturedFrame;
use ds_core::geometry::scale::Scale;
use smithay::backend::allocator::Format;
use smithay::backend::allocator::dmabuf::Dmabuf;

impl TtyBackend {
    /// Draws the capture frame of `output`, shown at `seq`: on the display GPU when it keeps
    /// scenes, else (or when that fails, one log line) in the software shadow. Nothing for an
    /// output nobody watches.
    pub(super) fn capture_draw(
        &mut self,
        output: &OutputName,
        seq: VblankSeq,
        at: Mono,
        source: &dyn FrameSource,
    ) -> Result<(), BackendError> {
        let Some(config) = self.readback.config_of(output) else {
            return Ok(());
        };
        let lender = self
            .devices
            .capture_gpu()
            .filter(|gpu| gpu.scenes() == SceneSupport::Drawn);
        let Some(mut gpu) = lender else {
            return self.readback.draw(output, seq, at, source);
        };
        let picture = Picture::of(&config);
        match gpu.draw_scene(output, &picture, source) {
            Ok(damage) => {
                let frame = PresentedFrame {
                    seq,
                    at,
                    pixels: FramePixels::Resident(picture.size),
                    damage,
                    bare: None,
                };
                self.readback.note_resident(output, frame);
                Ok(())
            }
            Err(error) => {
                tracing::info!(%error, "the display GPU did not draw the capture scene; using the software shadow");
                self.readback.forget_resident(output);
                self.readback.draw(output, seq, at, source)
            }
        }
    }

    /// Draws `output`'s first capture frame when it is watched and has none (the scene is
    /// still, so no flip drew one).
    pub(super) fn capture_first(
        &mut self,
        output: &OutputName,
        at: Mono,
        source: &dyn FrameSource,
    ) -> Result<(), BackendError> {
        match self.readback.last_frame(output) {
            None => self.capture_draw(output, VblankSeq(1), at, source),
            Some(_) => Ok(()),
        }
    }

    /// The device the display GPU draws on, when it can tell (its render node's number).
    pub fn capture_device(&self) -> Option<u64> {
        self.devices.capture_gpu()?.device()
    }

    /// How a closing window's last image is kept on this seat: on the display GPU when it is the
    /// seat's only device and lends its renderer, else read back.
    pub fn body_copy(&self) -> BodyCopy {
        let lending = self
            .devices
            .devices
            .values()
            .filter(|device| device.kms.capture_gpu().is_some())
            .count();
        body_copy(self.devices.devices.len(), lending)
    }

    /// Renders `source` alone into a texture on the display GPU that stays there (the last image
    /// of a closing window), its bottom corners cut by `radius` logical pixels. Only for a seat
    /// whose [`body_copy`](Self::body_copy) is [`BodyCopy::Gpu`]; the texture is drawn by the
    /// scanout's renderer, which is that GPU's.
    pub fn render_window_texture(
        &mut self,
        size: PhysicalSize,
        scale: Scale,
        radius: u32,
        source: &dyn FrameSource,
    ) -> Result<smithay::backend::renderer::gles::GlesTexture, BackendError> {
        match self.body_copy() {
            BodyCopy::Gpu => self
                .devices
                .capture_gpu()
                .ok_or(BackendError::NoFrame)?
                .render_window_texture(size, scale, radius, source),
            _ => Err(BackendError::NoFrame),
        }
    }

    /// Whether the display GPU can draw a capture into a dma-buf of format `wanted` (a window:
    /// the scene is rendered fresh). `false` when no device lends a renderer.
    pub fn binds_format(&self, wanted: Format) -> bool {
        self.devices
            .capture_gpu()
            .is_some_and(|gpu| matches!(capture_route(wanted, &gpu.formats()), CaptureRoute::Gpu))
    }

    /// Whether a capture of `output` can be drawn into a dma-buf of format `wanted`: the display
    /// GPU when the frame last drawn is on it, else the software shadow, which cannot.
    pub fn output_binds_format(&self, output: &OutputName, wanted: Format) -> bool {
        self.readback.on_gpu(output) && self.binds_format(wanted)
    }

    /// Renders `source` alone straight into a client's dma-buf `target` of exactly `size` with
    /// the display GPU: window capture with no read-back.
    pub fn render_window_dmabuf(
        &mut self,
        target: &mut Dmabuf,
        size: PhysicalSize,
        scale: Scale,
        source: &dyn FrameSource,
    ) -> Result<(), BackendError> {
        let mut gpu = self.devices.capture_gpu().ok_or(BackendError::NoFrame)?;
        gpu.render_window(target, size, scale, source)
    }

    /// Draws `from`, a part of the frame last drawn for `output` on the display GPU, into a
    /// client's dma-buf `target`. [`BackendError::NoFrame`] when that frame is not on the GPU.
    pub fn blit_output_dmabuf(
        &mut self,
        output: &OutputName,
        cursor: bool,
        from: PhysicalRect,
        target: &mut Dmabuf,
    ) -> Result<(), BackendError> {
        if !self.readback.on_gpu(output) {
            return Err(BackendError::NoFrame);
        }
        let mut gpu = self.devices.capture_gpu().ok_or(BackendError::NoFrame)?;
        gpu.blit_scene(output, cursor, from, target)
    }

    /// `area` of the frame last drawn for `output` on the display GPU, read back to the CPU (a
    /// shared-memory buffer). Only `area` is copied: a one-pixel pick reads one pixel.
    pub fn read_output(
        &mut self,
        output: &OutputName,
        cursor: bool,
        area: PhysicalRect,
    ) -> Result<CapturedFrame, BackendError> {
        let mut gpu = self.devices.capture_gpu().ok_or(BackendError::NoFrame)?;
        gpu.read_scene(output, cursor, area)
    }

    /// `output` is not watched any more: the display GPU frees its images.
    pub(super) fn capture_stopped(&mut self, output: &OutputName) {
        if let Some(mut gpu) = self.devices.capture_gpu() {
            gpu.forget_scene(output);
        }
    }
}
