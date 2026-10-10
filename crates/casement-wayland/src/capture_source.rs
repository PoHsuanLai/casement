//! `ext_foreign_toplevel_image_capture_source_manager_v1`: a client captures one window (the
//! screencast portal's window source, a window picker's thumbnails). P2-Q3.
//!
//! The source and its sessions are the same ext-image-copy-capture objects output capture
//! uses ([`crate::image_capture`]); this module keeps what is particular to a window:
//!
//! - the **size**: `round(logical × scale)` of the window's content on the output it is on
//!   ([`capture_size`]), announced as the session's constraints and announced again when the
//!   window's size changes (at its commit) or a frame is asked for after its output's scale
//!   changed;
//! - the **damage gate**: a frame request is held until the window's content changed since the
//!   last frame the session got (a [`ContentStamp`] that moves with every commit of the
//!   window's surface tree), so a static window costs a session no frames after its first;
//! - the **end**: a session whose window closes is stopped.
//!
//! The global is public (a portal is not the shell) and is created with the part. The pure
//! bookkeeping is [`ToplevelCaptureBook`] and the free functions, tested without a display; the
//! part adds smithay's state and the objects whose drop is a protocol event (a session's
//! `stopped`, a held frame's `failed`).

use crate::image_capture::CaptureFailure;
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::WindowId;
use casement_core::geometry::{LogicalSize, PhysicalSize, buffer_size};
use casement_core::policy::GlobalKind;
use casement_core::time::Mono;
use ds_core::geometry::scale::Scale;
use smithay::reexports::wayland_server::{Client, DisplayHandle};
use smithay::wayland::image_capture_source::{
    ToplevelCaptureSourceHandler, ToplevelCaptureSourceState,
};
use smithay::wayland::image_copy_capture::{Frame, Session};
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
    /// How many times each captured window's content changed (only windows with a session).
    pub stamps: BTreeMap<WindowId, ContentStamp>,
    /// The next id to hand out.
    pub next: u64,
}

impl ToplevelCaptureBook {
    /// The next id's number.
    fn mint(&mut self) -> u64 {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }

    /// The content `window` shows now.
    pub fn content_of(&self, window: WindowId) -> ContentStamp {
        self.stamps.get(&window).copied().unwrap_or_default()
    }

    /// The sessions that capture `window`.
    pub fn sessions_of(&self, window: WindowId) -> Vec<ToplevelSessionId> {
        self.sessions
            .iter()
            .filter(|(_, captured)| **captured == window)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Forgets a session and everything about it.
    pub fn drop_session(&mut self, session: ToplevelSessionId) {
        self.sessions.remove(&session);
        self.waiting.remove(&session);
        self.delivered.remove(&session);
    }

    /// The next frame of `session` must be drawn even if the content is the one it last got
    /// (its size changed, so the pixels it holds no longer fit).
    pub fn forget_delivered(&mut self, session: ToplevelSessionId) {
        self.delivered.remove(&session);
        self.waiting.remove(&session);
    }
}

/// The toplevel capture manager's state.
#[derive(Debug)]
pub struct ToplevelCapturePart {
    /// The sources, sessions and held frames.
    pub book: ToplevelCaptureBook,
    /// smithay's global (`ext_foreign_toplevel_image_capture_source_manager_v1`, version 1).
    pub sources: ToplevelCaptureSourceState,
    /// The sessions, held so that dropping one tells its client `stopped`.
    held: BTreeMap<ToplevelSessionId, Session>,
    /// The frame requests the damage gate holds; dropping one tells its client `failed`.
    frames: BTreeMap<ToplevelSessionId, Frame>,
}

impl ToplevelCapturePart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> ToplevelCapturePart
    where
        D: ToplevelCaptureSourceHandler + 'static,
    {
        // A concrete closure, as `ImageCapturePart::new` makes one.
        let filter = move |client: &Client| role_filter.sees(GlobalKind::ToplevelCapture, client);
        ToplevelCapturePart {
            book: ToplevelCaptureBook::default(),
            sources: ToplevelCaptureSourceState::new_with_filter::<D, _>(display, filter),
            held: BTreeMap::new(),
            frames: BTreeMap::new(),
        }
    }

    /// Keeps `session` until its client destroys it or its window closes.
    pub fn hold(&mut self, id: ToplevelSessionId, session: Session) {
        self.held.insert(id, session);
    }

    /// The session `id`, if it is still held.
    pub fn session(&self, id: ToplevelSessionId) -> Option<&Session> {
        self.held.get(&id)
    }

    /// Keeps `frame`, the request of `id` the damage gate holds (a request it replaces is
    /// dropped, which fails it).
    pub fn hold_frame(&mut self, id: ToplevelSessionId, frame: Frame) {
        self.frames.insert(id, frame);
    }

    /// The held request of `id`, to be answered.
    pub fn take_frame(&mut self, id: ToplevelSessionId) -> Option<Frame> {
        self.frames.remove(&id)
    }

    /// The client destroyed a session: forgets it (the dead session sends nothing).
    pub fn release(&mut self, id: ToplevelSessionId) {
        self.book.drop_session(id);
        self.held.remove(&id);
        self.frames.remove(&id);
    }

    /// `sessions` end: each is dropped, which tells its client `stopped` (and fails the frame
    /// it was held for).
    pub fn stop(&mut self, sessions: &[ToplevelSessionId]) {
        for id in sessions {
            // The session first: its drop fails the frame with `stopped`, and the frame's own
            // drop then has nothing left to say.
            self.held.remove(id);
            self.frames.remove(id);
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
///
/// ```
/// use casement_core::WindowId;
/// use casement_wayland::capture_source::{ToplevelCaptureBook, on_session, on_toplevel_source};
///
/// let mut book = ToplevelCaptureBook::default();
/// let source = on_toplevel_source(&mut book, WindowId(1));
/// assert!(on_session(&mut book, source).is_ok());
/// ```
pub fn on_toplevel_source(book: &mut ToplevelCaptureBook, window: WindowId) -> ToplevelSourceId {
    let id = ToplevelSourceId(book.mint());
    book.sources.insert(id, window);
    id
}

/// A client destroyed a source (its sessions go on).
pub fn on_source_destroyed(book: &mut ToplevelCaptureBook, source: ToplevelSourceId) {
    book.sources.remove(&source);
}

/// A client started a session on `source`.
pub fn on_session(
    book: &mut ToplevelCaptureBook,
    source: ToplevelSourceId,
) -> Result<ToplevelSessionId, WaylandFault> {
    let window = book
        .sources
        .get(&source)
        .copied()
        .ok_or(WaylandFault::UnknownCapture(source.0))?;
    let id = ToplevelSessionId(book.mint());
    book.sessions.insert(id, window);
    Ok(id)
}

/// A client asked for a frame of `session` into a buffer of `buffer`, the window now showing
/// `content`: the damage gate decides whether to render or to hold.
pub fn on_frame_request(
    book: &mut ToplevelCaptureBook,
    session: ToplevelSessionId,
    buffer: PhysicalSize,
    content: ContentStamp,
) -> Result<FrameVerdict, WaylandFault> {
    let window = book
        .sessions
        .get(&session)
        .copied()
        .ok_or(WaylandFault::UnknownCapture(session.0))?;
    let seen = book.delivered.get(&session).map(|(stamp, _)| *stamp);
    match seen == Some(content) {
        true => {
            book.waiting.insert(session, buffer);
            Ok(FrameVerdict::Hold)
        }
        false => Ok(FrameVerdict::Capture(ToplevelCaptureRequest {
            session,
            window,
            size: buffer,
            content,
        })),
    }
}

/// `window` committed: its content stamp moves (only a window with a session keeps one) and
/// the held requests that are news now come back, to be rendered.
pub fn on_commit(book: &mut ToplevelCaptureBook, window: WindowId) -> Vec<ToplevelCaptureRequest> {
    if book.sessions_of(window).is_empty() {
        return Vec::new();
    }
    let next = ContentStamp(book.content_of(window).0.saturating_add(1));
    book.stamps.insert(window, next);
    apply_content(book, window, next)
}

/// `window`'s content changed to `content`: the held requests of its sessions that this is
/// news to, to be rendered now.
pub fn apply_content(
    book: &mut ToplevelCaptureBook,
    window: WindowId,
    content: ContentStamp,
) -> Vec<ToplevelCaptureRequest> {
    let news: Vec<(ToplevelSessionId, PhysicalSize)> = book
        .waiting
        .iter()
        .filter(|(session, _)| book.sessions.get(*session) == Some(&window))
        .filter(|(session, _)| {
            book.delivered.get(*session).map(|(stamp, _)| *stamp) != Some(content)
        })
        .map(|(session, size)| (*session, *size))
        .collect();
    news.into_iter()
        .map(|(session, size)| {
            book.waiting.remove(&session);
            ToplevelCaptureRequest {
                session,
                window,
                size,
                content,
            }
        })
        .collect()
}

/// The frame of `session` (showing `content`) was copied into the client's buffer, presented
/// at `at`: `ready`.
pub fn apply_ready(
    book: &mut ToplevelCaptureBook,
    session: ToplevelSessionId,
    content: ContentStamp,
    at: Mono,
) -> Result<(), WaylandFault> {
    if !book.sessions.contains_key(&session) {
        return Err(WaylandFault::UnknownCapture(session.0));
    }
    book.waiting.remove(&session);
    book.delivered.insert(session, (content, at));
    Ok(())
}

/// The frame of `session` could not be made: `failed`. A session whose window is gone is over.
pub fn apply_failed(
    book: &mut ToplevelCaptureBook,
    session: ToplevelSessionId,
    why: CaptureFailure,
) -> Result<(), WaylandFault> {
    if !book.sessions.contains_key(&session) {
        return Err(WaylandFault::UnknownCapture(session.0));
    }
    book.waiting.remove(&session);
    if why == CaptureFailure::OutputGone {
        book.drop_session(session);
    }
    Ok(())
}

/// `window` closed: its sources and sessions end; the sessions to stop.
pub fn apply_window_gone(
    book: &mut ToplevelCaptureBook,
    window: WindowId,
) -> Vec<ToplevelSessionId> {
    book.sources.retain(|_, shown| *shown != window);
    book.stamps.remove(&window);
    let gone = book.sessions_of(window);
    gone.iter().for_each(|id| book.drop_session(*id));
    gone
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One row: case, logical size, scale in 120ths, physical size.
    type SizeRow = (&'static str, (u32, u32), u32, (u32, u32));

    const SIZES: &[SizeRow] = &[
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

    const SIZE: PhysicalSize = PhysicalSize { w: 801, h: 601 };

    /// A book with a source and a session on window 1.
    fn book_with_session() -> (ToplevelCaptureBook, ToplevelSessionId) {
        let mut book = ToplevelCaptureBook::default();
        let source = on_toplevel_source(&mut book, WindowId(1));
        let session = on_session(&mut book, source).expect("a session");
        (book, session)
    }

    fn at(n: u64) -> Mono {
        Mono(std::time::Duration::from_secs(n))
    }

    #[test]
    fn the_first_frame_is_drawn_and_the_same_content_is_held() {
        let (mut book, session) = book_with_session();
        let first = on_frame_request(&mut book, session, SIZE, ContentStamp(0)).expect("a verdict");
        assert!(matches!(first, FrameVerdict::Capture(_)));
        apply_ready(&mut book, session, ContentStamp(0), at(1)).expect("ready");
        let again = on_frame_request(&mut book, session, SIZE, ContentStamp(0)).expect("a verdict");
        assert_eq!(again, FrameVerdict::Hold);
        assert_eq!(book.waiting.get(&session), Some(&SIZE), "the request waits");
    }

    #[test]
    fn a_commit_releases_exactly_one_held_request() {
        let (mut book, session) = book_with_session();
        on_frame_request(&mut book, session, SIZE, ContentStamp(0)).expect("a verdict");
        apply_ready(&mut book, session, ContentStamp(0), at(1)).expect("ready");
        on_frame_request(&mut book, session, SIZE, ContentStamp(0)).expect("held");
        let released = on_commit(&mut book, WindowId(1));
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].session, session);
        assert_eq!(released[0].content, ContentStamp(1));
        assert!(
            on_commit(&mut book, WindowId(1)).is_empty(),
            "nothing waits now"
        );
    }

    #[test]
    fn a_window_nobody_captures_keeps_no_stamp() {
        let mut book = ToplevelCaptureBook::default();
        assert!(on_commit(&mut book, WindowId(9)).is_empty());
        assert!(book.stamps.is_empty());
    }

    #[test]
    fn a_closed_window_ends_its_sessions_and_sources() {
        let (mut book, session) = book_with_session();
        let gone = apply_window_gone(&mut book, WindowId(1));
        assert_eq!(gone, vec![session]);
        assert!(book.sources.is_empty() && book.sessions.is_empty());
        assert!(on_frame_request(&mut book, session, SIZE, ContentStamp(0)).is_err());
    }

    #[test]
    fn a_session_on_a_destroyed_source_is_refused_and_one_before_it_goes_on() {
        let mut book = ToplevelCaptureBook::default();
        let source = on_toplevel_source(&mut book, WindowId(2));
        let session = on_session(&mut book, source).expect("a session");
        on_source_destroyed(&mut book, source);
        assert!(on_session(&mut book, source).is_err());
        assert!(book.sessions.contains_key(&session));
    }

    #[test]
    fn a_resized_session_draws_again_at_the_same_content() {
        let (mut book, session) = book_with_session();
        apply_ready(&mut book, session, ContentStamp(0), at(1)).expect("ready");
        book.forget_delivered(session);
        let verdict = on_frame_request(&mut book, session, SIZE, ContentStamp(0)).expect("verdict");
        assert!(matches!(verdict, FrameVerdict::Capture(_)));
    }
}
