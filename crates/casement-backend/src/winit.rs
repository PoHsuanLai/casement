//! The winit backend: casement in a window of the user's own session, for the user's manual
//! feel checks (P0 exit, "nested winit feel"). Agents build it and never run it (§3.2); a
//! binary built with `testing` or `debug-inject` refuses it.

use crate::backend::BackendError;
use crate::headless::VirtualOutputSpec;

/// The nested window, its renderer and its input.
#[derive(Debug)]
pub struct WinitBackend {
    #[expect(dead_code, reason = "P0-B-loop: the winit graphics backend")]
    output: VirtualOutputSpec,
}

impl WinitBackend {
    /// Opens the window at `output`'s size and scale.
    pub fn start(output: VirtualOutputSpec) -> Result<WinitBackend, BackendError> {
        let _ = output;
        todo!("P0-B-loop: WinitBackend::start")
    }
}
