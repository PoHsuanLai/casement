//! `ext_output_image_capture_source_manager_v1` and `ext_image_copy_capture_manager_v1`: a
//! client captures an output (the screenshot tool, the screencast portal's stopgap). A capture
//! is the same frame scanout shows (byte-exact on pixman); toplevel sources arrive with P2.
//!
//! smithay owns the wire (buffer validation, the session's constraints, the frame's events);
//! this module keeps the [`CaptureBook`] (which source shows which output, which session has a
//! frame asked for) and the smithay [`Session`]s, whose drop is the `stopped` event.

mod book;

pub use book::CaptureBook;

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::OutputKey;
use casement_core::geometry::PhysicalSize;
use casement_core::policy::GlobalKind;
use casement_core::time::Mono;
use smithay::reexports::wayland_server::{Client, DisplayHandle};
use smithay::wayland::image_capture_source::{
    OutputCaptureSourceHandler, OutputCaptureSourceState,
};
use smithay::wayland::image_copy_capture::{
    CaptureFailureReason, Frame, ImageCopyCaptureHandler, ImageCopyCaptureState, Session,
    SessionRef,
};
use std::collections::BTreeMap;

/// One image capture source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaptureSourceId(pub u64);

/// One capture session on a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaptureSessionId(pub u64);

/// The managers' state: the book, smithay's two global states and the sessions held.
#[derive(Debug)]
pub struct ImageCapturePart {
    /// The sources made and the sessions running.
    pub book: CaptureBook,
    /// `ext_output_image_capture_source_manager_v1`.
    pub output_sources: OutputCaptureSourceState,
    /// `ext_image_copy_capture_manager_v1`.
    pub copy: ImageCopyCaptureState,
    /// The sessions, held so that dropping one tells its client `stopped`.
    held: BTreeMap<CaptureSessionId, Session>,
    /// The frame requests that found no frame presented yet; dropping one tells its client
    /// `failed`.
    waiting: BTreeMap<CaptureSessionId, Frame>,
}

impl ImageCapturePart {
    /// Creates the globals (version 1 each), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> ImageCapturePart
    where
        D: ImageCopyCaptureHandler + OutputCaptureSourceHandler + 'static,
    {
        // A concrete closure, not `for_global`'s opaque one: smithay wants `Clone` of the copy
        // manager's filter.
        let filter = move |client: &Client| role_filter.sees(GlobalKind::ImageCapture, client);
        ImageCapturePart {
            book: CaptureBook::default(),
            output_sources: OutputCaptureSourceState::new_with_filter::<D, _>(display, filter),
            copy: ImageCopyCaptureState::new_with_filter::<D, _>(display, filter),
            held: BTreeMap::new(),
            waiting: BTreeMap::new(),
        }
    }

    /// Keeps `session` until its client destroys it or its output goes.
    pub fn hold(&mut self, id: CaptureSessionId, session: Session) {
        self.held.insert(id, session);
    }

    /// Keeps `frame`, the request of `id` that found no frame to copy yet (a request it
    /// replaces is dropped, which fails it).
    pub fn wait_frame(&mut self, id: CaptureSessionId, frame: Frame) {
        self.waiting.insert(id, frame);
    }

    /// The waiting requests, to be served again.
    pub fn take_waiting(&mut self) -> Vec<(CaptureSessionId, Frame)> {
        std::mem::take(&mut self.waiting).into_iter().collect()
    }

    /// The session `id`, if it is still held.
    pub fn session_ref(&self, id: CaptureSessionId) -> Option<SessionRef> {
        self.held.get(&id).map(Session::as_ref)
    }

    /// The client destroyed a session: forgets it (the dead session sends nothing).
    pub fn release(&mut self, id: CaptureSessionId) {
        self.book.drop_session(id);
        self.waiting.remove(&id);
        self.held.remove(&id);
        self.copy.cleanup();
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

impl CaptureFailure {
    /// The reason the `failed` event carries.
    pub fn reason(self) -> CaptureFailureReason {
        match self {
            CaptureFailure::OutputGone => CaptureFailureReason::Stopped,
            CaptureFailure::BufferMismatch => CaptureFailureReason::BufferConstraints,
            CaptureFailure::Renderer => CaptureFailureReason::Unknown,
        }
    }
}

/// A client made a source for `output`.
///
/// ```no_run
/// use casement_wayland::image_capture::{CaptureBook, on_output_source};
///
/// # fn demo(key: casement_core::OutputKey) {
/// let mut book = CaptureBook::default();
/// let source = on_output_source(&mut book, key);
/// let _ = source;
/// # }
/// ```
pub fn on_output_source(book: &mut CaptureBook, output: OutputKey) -> CaptureSourceId {
    book.add_source(output)
}

/// A client destroyed a source.
pub fn on_source_destroyed(book: &mut CaptureBook, source: CaptureSourceId) {
    book.drop_source(source);
}

/// A client started a session on `source`.
pub fn on_session(
    book: &mut CaptureBook,
    source: CaptureSourceId,
) -> Result<CaptureSessionId, WaylandFault> {
    book.add_session(source)
}

/// A client asked for a frame: what to render.
pub fn on_frame_request(
    book: &mut CaptureBook,
    session: CaptureSessionId,
    buffer: PhysicalSize,
) -> Result<CaptureRequest, WaylandFault> {
    book.request_frame(session, buffer)
}

/// The frame was copied into the client's buffer, presented at `at`: `ready`.
pub fn apply_ready(
    book: &mut CaptureBook,
    session: CaptureSessionId,
    at: Mono,
) -> Result<(), WaylandFault> {
    book.frame_ready(session, at)
}

/// The frame could not be made: `failed`.
pub fn apply_failed(
    book: &mut CaptureBook,
    session: CaptureSessionId,
    why: CaptureFailure,
) -> Result<(), WaylandFault> {
    book.frame_failed(session, why)
}

/// `output` was unplugged: its sessions get `stopped` (the held session is dropped).
pub fn apply_output_gone(part: &mut ImageCapturePart, output: &OutputKey) {
    for id in part.book.forget_output(output) {
        // The session first: its drop fails the waiting frame with `stopped`.
        part.held.remove(&id);
        part.waiting.remove(&id);
    }
    part.copy.cleanup();
}
