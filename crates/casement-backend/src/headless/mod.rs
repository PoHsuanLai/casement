//! The headless backend: virtual outputs rendered offscreen, a vblank source on the session
//! clock, a render-cost model, and [`crate::VirtualInput`] on the same input path libinput
//! takes. What every test and every jailed run uses (11 §2).

mod cost;
mod input;
mod output_set;
mod renderer;
mod vblank;
mod virtual_output;

pub use cost::RenderCostModel;
pub use input::HeadlessInput;
pub use output_set::{OutputSet, VirtualOutput};
pub use renderer::{HeadlessRenderer, SoftRenderer};
pub use vblank::VblankSource;
pub use virtual_output::{EdidSource, FixtureName, GpuId, VirtualOutputSpec};

use crate::backend::BackendError;
use crate::input::VirtualInput;
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::output::{Mode, OutputFacts, OutputPower};
use casement_core::time::{Mono, VblankSeq};
use casement_core::{OutputKey, OutputName};
use ds_core::geometry::scale::Scale;

/// Whether clients are offered dma-buf (`--dmabuf none|soft`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DmabufMode {
    /// No `zwp_linux_dmabuf_v1` global: clients fall back to shm (shell-host F80).
    None,
    /// A fake main device over `/dev/udmabuf`.
    Soft,
}

/// How the headless backend starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessConfig {
    /// The outputs present at start.
    pub outputs: Vec<VirtualOutputSpec>,
    /// The renderer.
    pub renderer: SoftRenderer,
    /// Whether dma-buf is offered.
    pub dmabuf: DmabufMode,
    /// What a frame costs.
    pub cost: RenderCostModel,
}

/// The headless backend.
#[derive(Debug)]
pub struct HeadlessBackend {
    #[expect(dead_code, reason = "P0-B-headless: the virtual outputs")]
    outputs: OutputSet,
    #[expect(dead_code, reason = "P0-B-headless: the renderer")]
    renderer: HeadlessRenderer,
    #[expect(dead_code, reason = "P0-B-headless: the virtual input devices")]
    input: VirtualInput,
    #[expect(dead_code, reason = "P0-B-headless: the render-cost model")]
    cost: RenderCostModel,
}

impl HeadlessBackend {
    /// Starts the renderer and plugs `config`'s outputs (reported through [`HeadlessBackend::plug`]'s
    /// path by the caller, so start-up and hotplug are one code path).
    pub fn start(config: HeadlessConfig) -> Result<HeadlessBackend, BackendError> {
        let _ = config;
        todo!("P0-B-headless: HeadlessBackend::start")
    }

    /// Plugs a virtual output: the same event a KMS connector gives.
    pub fn plug(&mut self, spec: VirtualOutputSpec) -> ConnectorEvent {
        let _ = spec;
        todo!("P0-B-headless: HeadlessBackend::plug")
    }

    /// Unplugs `name`.
    pub fn unplug(&mut self, name: &OutputName) -> ConnectorEvent {
        let _ = name;
        todo!("P0-B-headless: HeadlessBackend::unplug")
    }

    /// Changes `name`'s mode, scale, position or power.
    pub fn set(&mut self, name: &OutputName, change: OutputChange) -> ConnectorEvent {
        let _ = (name, change);
        todo!("P0-B-headless: HeadlessBackend::set")
    }

    /// The last frame `name` presented (screenshots and capture).
    pub fn last_frame(&self, name: &OutputName) -> Option<&PresentedFrame> {
        let _ = name;
        todo!("P0-B-headless: HeadlessBackend::last_frame")
    }

    /// The virtual input devices.
    pub fn input(&mut self) -> &mut VirtualInput {
        todo!("P0-B-headless: HeadlessBackend::input")
    }
}

/// A change to a plugged output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputChange {
    /// A new mode.
    Mode(Mode),
    /// A new scale.
    Scale(Scale),
    /// A new position in the global logical space.
    Position(LogicalPoint),
    /// Powered on or off.
    Power(OutputPower),
}

/// What a connector did: the one event shape headless and KMS both produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectorEvent {
    /// A display appeared.
    Plugged(OutputFacts),
    /// A display went away.
    Unplugged(OutputKey),
    /// A display's facts changed.
    Changed(OutputFacts),
}

/// One frame an output presented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentedFrame {
    /// The vblank it was shown at.
    pub seq: VblankSeq,
    /// When.
    pub at: Mono,
    /// Its pixels.
    pub pixels: FramePixels,
}

/// A presented frame's pixels. Every renderer reads back `Abgr8888` (RGBA byte order), so
/// pixman and llvmpipe frames compare byte for byte (SP2). (The plan's sketch held renderer
/// images; a read-back is what screenshots and goldens use: FINDINGS.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramePixels {
    /// Read back from pixman or GLES.
    Rgba(RgbaFrame),
    /// The dummy renderer draws nothing.
    Dummy,
}

/// Read-back pixels: rows top to bottom, four bytes per pixel in R, G, B, A order,
/// premultiplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaFrame {
    /// The size in physical pixels.
    pub size: PhysicalSize,
    /// `size.w * size.h * 4` bytes.
    pub bytes: Vec<u8>,
}
