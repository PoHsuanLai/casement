//! Capture on a real seat (P1-T): the pixels of what an output shows, for
//! `ext-image-copy-capture`.
//!
//! A frame that goes to the screen through a device's own renderer and the kernel's framebuffers
//! cannot be read back without a second render, and a read-back of every frame is a cost nobody
//! pays unless a client is capturing. So while a capture session is open on an output the
//! backend draws the same scene a second time through a software renderer (pixman, the headless
//! backend's own) into a shadow output of the same mode, scale and transform, and keeps the last
//! frame; with no session nothing is drawn. The shadow is what the headless backend presents, so
//! a capture on KMS and a capture in the jail are one code path. A client buffer pixman cannot
//! import (a dma-buf of a GPU) is missing from the shadow: the departure is recorded in
//! FINDINGS (p1-t).

use crate::backend::BackendError;
use crate::headless::{
    DmabufMode, EdidSource, FrameSource, GpuId, HeadlessBackend, HeadlessConfig, PresentedFrame,
    RenderCostModel, RgbaFrame, SoftRenderer, VirtualOutputSpec,
};
use casement_core::OutputName;
use casement_core::geometry::PhysicalSize;
use casement_core::output::{EdidFacts, EdidSerial, OutputConfig, VrrCapability};
use casement_core::time::{Mono, VblankSeq};
use ds_core::geometry::scale::Scale;
use std::collections::BTreeMap;

/// Whether a capture is open on an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Watch {
    /// No capture: nothing is drawn for it.
    Off,
    /// A capture session is open: every frame shown is drawn again into the shadow.
    On,
}

/// Where the frame last drawn for an output is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Residence {
    /// On the display GPU: a capture draws it into a dma-buf there.
    Gpu,
    /// Read back by the software shadow (or not drawn at all).
    Shadow,
}

/// The shadow outputs, drawn only while watched.
#[derive(Debug, Default)]
pub struct Readback {
    /// Started on the first watch (a seat nobody captures never starts a renderer).
    shadow: Option<HeadlessBackend>,
    /// The outputs watched, and the configuration their shadow was made with.
    watched: BTreeMap<OutputName, OutputConfig>,
    /// The frames the display GPU kept on the GPU instead of the shadow reading them back
    /// (`TtyBackend::present` fills it while a lender keeps scenes); they win over the shadow's.
    resident: BTreeMap<OutputName, PresentedFrame>,
}

impl Readback {
    /// Nothing watched.
    pub fn new() -> Readback {
        Readback::default()
    }

    /// Starts or stops watching `output`, shown with `config`. A watched output whose mode,
    /// scale or transform changed gets a new shadow.
    pub fn watch(
        &mut self,
        output: &OutputName,
        config: &OutputConfig,
        watch: Watch,
    ) -> Result<(), BackendError> {
        match watch {
            Watch::Off => {
                self.resident.remove(output);
                if self.watched.remove(output).is_some()
                    && let Some(shadow) = &mut self.shadow
                {
                    let _ = shadow.unplug(output);
                }
                Ok(())
            }
            Watch::On => self.follow(output, config),
        }
    }

    /// Makes sure the shadow of `output` is made for `config`.
    fn follow(&mut self, output: &OutputName, config: &OutputConfig) -> Result<(), BackendError> {
        if self.watched.get(output) == Some(config) {
            return Ok(());
        }
        let shadow = self.started()?;
        let _ = shadow.plug(spec_of(output, config));
        self.watched.insert(output.clone(), *config);
        self.resident.remove(output);
        Ok(())
    }

    /// The shadow renderer, started if it was not.
    fn started(&mut self) -> Result<&mut HeadlessBackend, BackendError> {
        if self.shadow.is_none() {
            self.shadow = Some(HeadlessBackend::start(HeadlessConfig {
                outputs: Vec::new(),
                renderer: SoftRenderer::Pixman,
                dmabuf: DmabufMode::None,
                cost: RenderCostModel::Fixed(std::time::Duration::ZERO),
            })?);
        }
        self.shadow.as_mut().ok_or(BackendError::CanvasMismatch)
    }

    /// Draws `source` alone into a fresh image of `size` at `scale` through the software
    /// renderer and reads it back (window capture: no output needs to be watched).
    pub fn render_offscreen(
        &mut self,
        size: PhysicalSize,
        scale: Scale,
        source: &dyn FrameSource,
    ) -> Result<RgbaFrame, BackendError> {
        self.started()?.render_offscreen(size, scale, source)
    }

    /// The dma-buf formats a capture can be drawn into: the shadow's renderer is pixman, which
    /// binds linear buffers only.
    pub fn capture_formats(&self) -> Vec<smithay::backend::allocator::Format> {
        match &self.shadow {
            Some(shadow) => shadow.capture_formats(),
            None => crate::headless::linear_capture_formats(),
        }
    }

    /// Draws `frame` into a client's dma-buf `target` through the shadow's renderer.
    pub fn write_dmabuf(
        &mut self,
        target: &mut smithay::backend::allocator::dmabuf::Dmabuf,
        frame: &casement_render::pass::CapturedFrame,
    ) -> Result<(), BackendError> {
        self.started()?.write_dmabuf(target, frame)
    }

    /// Reads a client's dma-buf `source` back through the shadow's renderer.
    pub fn read_dmabuf(
        &mut self,
        source: &smithay::backend::allocator::dmabuf::Dmabuf,
    ) -> Result<casement_render::pass::ArgbImage, BackendError> {
        self.started()?.read_dmabuf(source)
    }

    /// Draws `source` into the shadow of `output` as the frame shown at `seq`; nothing when the
    /// output is not watched.
    pub fn draw(
        &mut self,
        output: &OutputName,
        seq: VblankSeq,
        at: Mono,
        source: &dyn FrameSource,
    ) -> Result<(), BackendError> {
        if !self.watched.contains_key(output) {
            return Ok(());
        }
        match &mut self.shadow {
            Some(shadow) => shadow.present(output, seq, at, source).map(|_| ()),
            None => Ok(()),
        }
    }

    /// Draws `source` into the shadow of `output` when it is watched and has no frame yet (the
    /// scene is unchanged, so no flip drew one).
    pub fn draw_first(
        &mut self,
        output: &OutputName,
        at: Mono,
        source: &dyn FrameSource,
    ) -> Result<(), BackendError> {
        match self.last_frame(output) {
            None => self.draw(output, VblankSeq(1), at, source),
            Some(_) => Ok(()),
        }
    }

    /// The frame last drawn for `output`: the one kept on the GPU when there is one, else the
    /// shadow's.
    pub fn last_frame(&self, output: &OutputName) -> Option<&PresentedFrame> {
        self.resident
            .get(output)
            .or_else(|| self.shadow.as_ref()?.last_frame(output))
    }

    /// The configuration `output` is watched with; none when it is not watched.
    pub fn config_of(&self, output: &OutputName) -> Option<OutputConfig> {
        self.watched.get(output).copied()
    }

    /// The display GPU drew `frame` for `output` and kept it on the GPU.
    pub fn note_resident(&mut self, output: &OutputName, frame: PresentedFrame) {
        self.resident.insert(output.clone(), frame);
    }

    /// The frame kept on the GPU for `output` is stale (the GPU did not draw this one): the
    /// shadow's takes over.
    pub fn forget_resident(&mut self, output: &OutputName) {
        self.resident.remove(output);
    }

    /// Where the frame last drawn for `output` is.
    pub fn residence(&self, output: &OutputName) -> Residence {
        match self.resident.contains_key(output) {
            true => Residence::Gpu,
            false => Residence::Shadow,
        }
    }

    /// Whether `output` is watched.
    pub fn watching(&self, output: &OutputName) -> Watch {
        match self.watched.contains_key(output) {
            true => Watch::On,
            false => Watch::Off,
        }
    }

    /// `output` is gone: its shadow with it.
    pub fn forget(&mut self, output: &OutputName) {
        self.watched.remove(output);
        self.resident.remove(output);
        if let Some(shadow) = &mut self.shadow {
            let _ = shadow.unplug(output);
        }
    }
}

/// The virtual output that shows what `output` shows with `config`.
fn spec_of(output: &OutputName, config: &OutputConfig) -> VirtualOutputSpec {
    VirtualOutputSpec {
        name: output.clone(),
        mode: config.mode,
        scale: config.scale,
        transform: config.transform,
        position: config.position,
        gpu: GpuId(0),
        vrr: VrrCapability::Incapable,
        edid: EdidSource::Synthetic(EdidFacts {
            make: "Casement".to_owned(),
            model: "Shadow".to_owned(),
            serial: EdidSerial::Unknown,
        }),
    }
}
