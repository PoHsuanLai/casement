//! Which backend runs.

use crate::gpu::GpuError;
use crate::headless::{ConnectorEvent, FrameSource, HeadlessBackend, RgbaFrame, SpecError};
use crate::session::SessionFault;
use crate::tty::{KmsError, TtyBackend};
use crate::winit::WinitBackend;
use casement_core::OutputName;
use casement_core::geometry::{PhysicalRect, PhysicalSize};
use casement_render::RenderError;
use casement_render::pass::{ArgbImage, CapturedFrame};
use ds_core::geometry::scale::Scale;
use smithay::backend::SwapBuffersError;
use smithay::backend::allocator::Format;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::renderer::gles::GlesError;
use smithay::backend::renderer::pixman::PixmanError;
use std::path::PathBuf;

/// The running backend.
#[derive(Debug)]
#[non_exhaustive]
pub enum BackendKind {
    /// Virtual outputs and input: every test, every agent run.
    Headless(HeadlessBackend),
    /// A window on the user's own session: the user's feel checks only (never an agent, §3.2).
    Winit(Box<WinitBackend>),
    /// A real seat: DRM devices, libseat, udev (P1; the seams are injected, so the FakeKms
    /// suite runs it in the jail).
    Tty(Box<TtyBackend>),
}

impl BackendKind {
    /// The running backend as the verbs every backend shares.
    pub fn driver(&self) -> &dyn Backend {
        match self {
            BackendKind::Headless(backend) => backend,
            BackendKind::Winit(window) => &**window,
            BackendKind::Tty(tty) => &**tty,
        }
    }

    /// The running backend as the verbs every backend shares, mutably.
    pub fn driver_mut(&mut self) -> &mut dyn Backend {
        match self {
            BackendKind::Headless(backend) => backend,
            BackendKind::Winit(window) => &mut **window,
            BackendKind::Tty(tty) => &mut **tty,
        }
    }
}

/// The verbs the headless, winit and tty backends share. The defaults are the answers of a
/// backend with no renderer of its own to draw a capture with (winit): nothing to offer and every
/// draw refused. A call site that needs one backend's own methods matches [`BackendKind`] instead.
pub trait Backend {
    /// The connectors plugged at start, once (start-up and hotplug are one code path).
    fn announce(&mut self) -> Vec<ConnectorEvent>;

    /// The dma-buf formats a capture can be drawn into.
    fn capture_formats(&self) -> Vec<Format> {
        Vec::new()
    }

    /// Draws `frame` into a client's dma-buf `target` (a capture's buffer).
    fn write_dmabuf(
        &mut self,
        _target: &mut Dmabuf,
        _frame: &CapturedFrame,
    ) -> Result<(), BackendError> {
        Err(RenderError::NoTarget.into())
    }

    /// Reads a client's dma-buf `source` (a cursor surface's buffer) back.
    fn read_dmabuf(&mut self, _source: &Dmabuf) -> Result<ArgbImage, BackendError> {
        Err(RenderError::NoTarget.into())
    }

    /// Draws `source` alone into a fresh image of `size` at `scale` and reads it back (window
    /// capture).
    fn render_offscreen(
        &mut self,
        _size: PhysicalSize,
        _scale: Scale,
        _source: &dyn FrameSource,
    ) -> Result<RgbaFrame, BackendError> {
        Err(RenderError::NoTarget.into())
    }

    /// Renders `source` alone straight into a client's dma-buf `target`: window capture with no
    /// read-back.
    fn render_window_dmabuf(
        &mut self,
        _target: &mut Dmabuf,
        _size: PhysicalSize,
        _scale: Scale,
        _source: &dyn FrameSource,
    ) -> Result<(), BackendError> {
        Err(BackendError::NoFrame)
    }

    /// Blits `from`, a part of the frame `output` last presented (with its cursor when `cursor`),
    /// into a client's dma-buf `target`.
    fn blit_output_dmabuf(
        &mut self,
        _output: &OutputName,
        _cursor: bool,
        _from: PhysicalRect,
        _target: &mut Dmabuf,
    ) -> Result<(), BackendError> {
        Err(BackendError::NoFrame)
    }

    /// The steps of `output`'s gamma table; none when the backend has no gamma to drive.
    fn gamma_steps(&self, _output: &OutputName) -> Option<u32> {
        None
    }
}

/// The backend `--backend` asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendChoice {
    /// `headless`.
    Headless,
    /// `winit`.
    Winit,
    /// `tty`.
    Tty,
}

/// Why a backend could not start or act.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BackendError {
    /// An output spec did not parse.
    #[error("bad output spec {text}: {why}")]
    BadSpec {
        /// The spec as given.
        text: String,
        /// What was wrong with it.
        #[source]
        why: SpecError,
    },
    /// The nested window could not start.
    #[error("window: {0}")]
    Window(#[from] smithay::backend::winit::Error),
    /// The nested window could not bind or swap its buffers.
    #[error("swap buffers: {0}")]
    Swap(#[from] SwapBuffersError),
    /// The GLES renderer refused.
    #[error("gles renderer: {0}")]
    Gles(#[from] GlesError),
    /// The pixman renderer refused.
    #[error("pixman renderer: {0}")]
    Pixman(#[from] PixmanError),
    /// Drawing a frame failed.
    #[error("render: {0}")]
    Render(#[from] RenderError),
    /// The dummy renderer was asked for without the `testing` feature.
    #[error("the dummy renderer needs the `testing` feature")]
    DummyNeedsTesting,
    /// An output's canvas was made by another renderer than the one drawing into it.
    #[error("an output's canvas belongs to another renderer")]
    CanvasMismatch,
    /// An output has drawn no frame (or none that stayed on the GPU) to copy.
    #[error("the output has no frame to copy")]
    NoFrame,
    /// An output's mode does not fit smithay's buffer sizes.
    #[error("a {w}x{h} output is too large")]
    OutputTooLarge {
        /// The mode's width.
        w: u32,
        /// The mode's height.
        h: u32,
    },
    /// A display device refused.
    #[error("kms: {0}")]
    Kms(#[from] KmsError),
    /// The session refused.
    #[error("session: {0}")]
    Session(#[from] SessionFault),
    /// A GPU call failed.
    #[error("gpu: {0}")]
    Gpu(#[from] GpuError),
    /// A device open was refused (in the jail: anything but the fixtures and /dev/udmabuf).
    #[error("denied open {}: {source}", path.display())]
    DeniedOpen {
        /// The device that was asked for.
        path: PathBuf,
        /// Why the open failed.
        #[source]
        source: std::io::Error,
    },
}
