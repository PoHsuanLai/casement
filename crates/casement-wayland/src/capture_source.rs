//! `ext_foreign_toplevel_image_capture_source_manager_v1`: a client captures one window (the
//! screencast portal's window source, a window picker's thumbnails). Frozen at P2-Z; filled by
//! P2-Q3.
//!
//! The source and its sessions are the same ext-image-copy-capture objects output capture
//! uses ([`crate::image_capture`]); this module keeps what is particular to a window:
//!
//! - the **size**: `round(logical × scale)` of the window's content on the output it is on
//!   ([`capture_size`]), announced as the session's constraints and announced again when the
//!   window or its output's scale changes;
//! - the **damage gate**: a frame request is held until the window's content changed since the
//!   last frame the session got (a [`ContentStamp`] that moves with every commit of the
//!   window's surface tree), so a static window costs a session no frames after its first;
//! - the **end**: a session whose window closes is stopped.
//!
//! The global is not created yet ([`ToplevelCapturePart::new`] only keeps the filter): it would
//! be public (a portal is not the shell), and a source made on it would reach a `todo!()`.
//! P2-Q3 adds smithay's `ToplevelCaptureSourceState` to the part, implements
//! `ToplevelCaptureSourceHandler` in `casement/src/handlers/capture_source.rs` and creates the
//! global.

use crate::image_capture::CaptureFailure;
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::WindowId;
use casement_core::geometry::{LogicalSize, PhysicalSize, buffer_size};
use casement_core::time::Mono;
use ds_core::geometry::scale::Scale;
use smithay::reexports::wayland_server::DisplayHandle;
use smithay::wayland::image_capture_source::ImageCaptureSourceHandler;
use std::collections::BTreeMap;

/// One toplevel capture source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToplevelSourceId(pub u64);

/// One capture session on a toplevel source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToplevelSessionId(pub u64);

/// How many times a window's content has changed: the count of commits of its surface tree
/// (the toplevel and its subsurfaces). Two equal stamps are the same pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct ContentStamp(pub u64);

/// What a frame request asks the renderer for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ToplevelCaptureRequest {
    /// The session.
    pub session: ToplevelSessionId,
    /// The window to capture.
    pub window: WindowId,
    /// The size of the buffer the client attached, in physical pixels.
    pub size: PhysicalSize,
    /// The content the frame will show.
    pub content: ContentStamp,
}

/// What the damage gate decided for a frame request.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrameVerdict {
    /// The content changed since the session's last frame: render this.
    Capture(ToplevelCaptureRequest),
    /// Nothing changed: the request waits (no `ready`, no `failed`) until
    /// [`apply_content`] reports damage.
    Hold,
}

/// The sources made, the sessions running and the frames waiting.
#[derive(Debug, Default)]
pub struct ToplevelCaptureBook {
    /// Each source and the window it shows.
    pub sources: BTreeMap<ToplevelSourceId, WindowId>,
    /// Each session and the window it captures (a session outlives its source).
    pub sessions: BTreeMap<ToplevelSessionId, WindowId>,
    /// Each session whose frame request is held, and the buffer size it came with.
    pub waiting: BTreeMap<ToplevelSessionId, PhysicalSize>,
    /// The content each session was last given and when.
    pub delivered: BTreeMap<ToplevelSessionId, (ContentStamp, Mono)>,
    /// The next id to hand out.
    pub next: u64,
}

/// The toplevel capture manager's state.
#[derive(Debug)]
pub struct ToplevelCapturePart {
    /// The sources, sessions and held frames.
    pub book: ToplevelCaptureBook,
    /// The filter the global will be created behind (`GlobalKind::ToplevelCapture`).
    pub filter: GlobalFilter,
}

impl ToplevelCapturePart {
    /// The part. Creates no global at the freeze (see the module); P2-Q3 tightens the bound to
    /// `ToplevelCaptureSourceHandler` and creates smithay's state with the filter.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> ToplevelCapturePart
    where
        D: ImageCaptureSourceHandler + 'static,
    {
        let _ = display;
        ToplevelCapturePart {
            book: ToplevelCaptureBook::default(),
            filter: role_filter,
        }
    }
}

/// The buffer a capture of a window with `logical` content needs at `scale`: each side
/// `round(l × s)`, the same conversion every buffer uses.
///
/// ```
/// use casement_core::geometry::LogicalSize;
/// use casement_wayland::capture_source::capture_size;
/// use ds_core::geometry::scale::Scale;
///
/// let size = capture_size(LogicalSize { w: 801, h: 601 }, Scale(150));
/// assert_eq!((size.w, size.h), (1_001, 751));
/// ```
pub fn capture_size(logical: LogicalSize, scale: Scale) -> PhysicalSize {
    buffer_size(logical, scale)
}

/// A client made a source for `window`.
pub fn on_toplevel_source(book: &mut ToplevelCaptureBook, window: WindowId) -> ToplevelSourceId {
    let _ = (book, window);
    todo!("P2-Q3: on_toplevel_source")
}

/// A client destroyed a source (its sessions go on).
pub fn on_source_destroyed(book: &mut ToplevelCaptureBook, source: ToplevelSourceId) {
    let _ = (book, source);
    todo!("P2-Q3: on_source_destroyed")
}

/// A client started a session on `source`.
pub fn on_session(
    book: &mut ToplevelCaptureBook,
    source: ToplevelSourceId,
) -> Result<ToplevelSessionId, WaylandFault> {
    let _ = (book, source);
    todo!("P2-Q3: on_session")
}

/// A client asked for a frame of `session` into a buffer of `buffer`, the window now showing
/// `content`: the damage gate decides whether to render or to hold.
pub fn on_frame_request(
    book: &mut ToplevelCaptureBook,
    session: ToplevelSessionId,
    buffer: PhysicalSize,
    content: ContentStamp,
) -> Result<FrameVerdict, WaylandFault> {
    let _ = (book, session, buffer, content);
    todo!("P2-Q3: on_frame_request")
}

/// `window`'s content changed to `content`: the held requests of its sessions that this is
/// news to, to be rendered now.
pub fn apply_content(
    book: &mut ToplevelCaptureBook,
    window: WindowId,
    content: ContentStamp,
) -> Vec<ToplevelCaptureRequest> {
    let _ = (book, window, content);
    todo!("P2-Q3: apply_content")
}

/// The frame of `session` (showing `content`) was copied into the client's buffer, presented
/// at `at`: `ready`.
pub fn apply_ready(
    book: &mut ToplevelCaptureBook,
    session: ToplevelSessionId,
    content: ContentStamp,
    at: Mono,
) -> Result<(), WaylandFault> {
    let _ = (book, session, content, at);
    todo!("P2-Q3: apply_ready")
}

/// The frame of `session` could not be made: `failed`.
pub fn apply_failed(
    book: &mut ToplevelCaptureBook,
    session: ToplevelSessionId,
    why: CaptureFailure,
) -> Result<(), WaylandFault> {
    let _ = (book, session, why);
    todo!("P2-Q3: apply_failed")
}

/// `window` closed: its sources and sessions end; the sessions to stop.
pub fn apply_window_gone(
    book: &mut ToplevelCaptureBook,
    window: WindowId,
) -> Vec<ToplevelSessionId> {
    let _ = (book, window);
    todo!("P2-Q3: apply_window_gone")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (case, logical, scale in 120ths, physical)
    const SIZES: &[(&str, (u32, u32), u32, (u32, u32))] = &[
        ("1x", (800, 600), 120, (800, 600)),
        ("1.25x rounds each side", (801, 601), 150, (1_001, 751)),
        ("1.5x", (801, 601), 180, (1_202, 902)),
        ("2x", (801, 601), 240, (1_602, 1_202)),
    ];

    #[test]
    fn a_capture_is_the_window_rounded_at_the_output_scale() {
        for (case, (w, h), scale, (want_w, want_h)) in SIZES {
            let size = capture_size(LogicalSize { w: *w, h: *h }, Scale(*scale));
            assert_eq!((size.w, size.h), (*want_w, *want_h), "{case}");
        }
    }
}
