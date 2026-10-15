//! Which backend runs.

use crate::gpu::GpuError;
use crate::headless::{HeadlessBackend, SpecError};
use crate::session::SessionFault;
use crate::tty::{KmsError, TtyBackend};
use crate::winit::WinitBackend;
use casement_render::RenderError;
use smithay::backend::SwapBuffersError;
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
