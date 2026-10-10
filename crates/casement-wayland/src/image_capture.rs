//! `ext_output_image_capture_source_manager_v1` and `ext_image_copy_capture_manager_v1`: a
//! client captures an output (the screenshot tool, the screencast portal's stopgap). P1-Z
//! declares it; P1-C1 creates the globals and fills the functions. A capture is the same
//! frame scanout shows (byte-exact on pixman); toplevel sources arrive with P2.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::OutputKey;
use casement_core::geometry::PhysicalSize;
use casement_core::time::Mono;
use smithay::reexports::wayland_server::DisplayHandle;
use std::collections::BTreeMap;

/// One image capture source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaptureSourceId(pub u64);

/// One capture session on a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaptureSessionId(pub u64);

/// The managers' state: the sources made and the sessions running.
#[derive(Debug, Default)]
pub struct ImageCapturePart {
    /// Each source and the output it shows.
    pub sources: BTreeMap<CaptureSourceId, OutputKey>,
    /// Each session and its source.
    pub sessions: BTreeMap<CaptureSessionId, CaptureSourceId>,
    /// The next id to hand out.
    pub next: u64,
}

impl ImageCapturePart {
    /// Creates the globals (version 1 each), filtered by the I7 table. P1-Z: creates nothing.
    pub fn new<D: 'static>(display: &DisplayHandle, role_filter: GlobalFilter) -> ImageCapturePart {
        let _ = (display, role_filter, std::marker::PhantomData::<D>);
        ImageCapturePart::default()
    }
}

/// What a frame request asks the renderer for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaptureRequest {
    /// The output to capture.
    pub output: OutputKey,
    /// The size of the buffer the client attached, in the output's physical pixels.
    pub size: PhysicalSize,
}

/// Why a capture frame failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CaptureFailure {
    /// The output went away.
    OutputGone,
    /// The client's buffer does not match the session's constraints.
    BufferMismatch,
    /// The renderer could not read the frame back.
    Renderer,
}

/// A client made a source for `output`.
///
/// ```no_run
/// use casement_wayland::image_capture::{ImageCapturePart, on_output_source};
///
/// # fn demo(key: casement_core::OutputKey) {
/// let mut part = ImageCapturePart::default();
/// let source = on_output_source(&mut part, key);
/// let _ = source;
/// # }
/// ```
pub fn on_output_source(part: &mut ImageCapturePart, output: OutputKey) -> CaptureSourceId {
    let _ = (part, output);
    todo!("P1-C1: image_capture::on_output_source")
}

/// A client started a session on `source`.
pub fn on_session(
    part: &mut ImageCapturePart,
    source: CaptureSourceId,
) -> Result<CaptureSessionId, WaylandFault> {
    let _ = (part, source);
    todo!("P1-C1: image_capture::on_session")
}

/// A client asked for a frame: what to render.
pub fn on_frame_request(
    part: &mut ImageCapturePart,
    session: CaptureSessionId,
    buffer: PhysicalSize,
) -> Result<CaptureRequest, WaylandFault> {
    let _ = (part, session, buffer);
    todo!("P1-C1: image_capture::on_frame_request")
}

/// The frame was copied into the client's buffer, presented at `at`: `ready`.
pub fn apply_ready(
    part: &mut ImageCapturePart,
    session: CaptureSessionId,
    at: Mono,
) -> Result<(), WaylandFault> {
    let _ = (part, session, at);
    todo!("P1-C1: image_capture::apply_ready")
}

/// The frame could not be made: `failed`.
pub fn apply_failed(
    part: &mut ImageCapturePart,
    session: CaptureSessionId,
    why: CaptureFailure,
) -> Result<(), WaylandFault> {
    let _ = (part, session, why);
    todo!("P1-C1: image_capture::apply_failed")
}

/// `output` was unplugged: its sessions get `stopped`.
pub fn apply_output_gone(part: &mut ImageCapturePart, output: &OutputKey) {
    let _ = (part, output);
    todo!("P1-C1: image_capture::apply_output_gone")
}
