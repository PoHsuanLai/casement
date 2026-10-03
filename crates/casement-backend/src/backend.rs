//! Which backend runs.

use crate::headless::HeadlessBackend;
use crate::winit::WinitBackend;

/// The running backend. (`Tty(TtyBackend)` joins at P1-Z.)
#[derive(Debug)]
pub enum BackendKind {
    /// Virtual outputs and input: every test, every agent run.
    Headless(HeadlessBackend),
    /// A window on the user's own session: the user's feel checks only (never an agent, §3.2).
    Winit(WinitBackend),
}

/// The backend `--backend` asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendChoice {
    /// `headless`.
    Headless,
    /// `winit`.
    Winit,
    /// `tty` (P1; refused before P1-Z).
    Tty,
}

/// Why a backend could not start or act.
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    /// An output spec did not parse.
    #[error("bad output spec {0}")]
    BadSpec(String),
    /// The renderer could not start or render.
    #[error("renderer: {0}")]
    Renderer(String),
    /// A device open was refused (in the jail: anything but the fixtures and /dev/udmabuf).
    #[error("denied open {0}")]
    DeniedOpen(String),
}
