//! Toplevel image capture sources (P2-Z; filled by P2-Q3): the session half of
//! `ext_foreign_toplevel_image_capture_source_manager_v1`. The sources and sessions are the
//! ext-image-copy-capture objects `handlers/image_capture.rs` already serves; that file hands
//! the ones that belong to a window here (a source whose user data holds a `ToplevelSourceId`,
//! a session whose user data holds a `ToplevelSessionId`). This file also implements smithay's
//! `ToplevelCaptureSourceHandler` for the state.
//!
//! A frame is the window's surface tree drawn alone into an offscreen image ([`render`]), so
//! what overlaps the window on screen, an off-screen window and a scale that differs from the
//! output's change nothing (FINDINGS, p2-q3). The damage gate holds a request until the
//! window commits ([`committed`]).

mod render;

use crate::state::State;
use casement_core::WindowId;
use casement_core::geometry::LogicalSize;
use casement_core::window::SpacePlace;
use casement_render::pass::window_capture_size;
use casement_wayland::capture_source::{
    self, FrameVerdict, ToplevelCaptureRequest, ToplevelSessionId, ToplevelSourceId,
};
use casement_wayland::foreign_toplevel_list::window_of_identifier;
use casement_wayland::image_capture::CaptureFailure;
use ds_core::geometry::scale::Scale;
use render::draw_window;
pub(crate) use render::{body_copy, window_rgba, window_texture};
use smithay::reexports::wayland_server::protocol::wl_shm::Format;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Buffer, Rectangle, Size, Transform};
use smithay::wayland::foreign_toplevel_list::ForeignToplevelHandle;
use smithay::wayland::image_capture_source::{
    ImageCaptureSource, ToplevelCaptureSourceHandler, ToplevelCaptureSourceState,
};
use smithay::wayland::image_copy_capture::{BufferConstraints, Frame, Session, SessionRef};

impl ToplevelCaptureSourceHandler for State {
    fn toplevel_capture_source_state(&mut self) -> &mut ToplevelCaptureSourceState {
        &mut self.wl.capture_source.sources
    }

    /// A source of a window casement lists. A handle whose window is gone (or never was ours)
    /// leaves the source without an id, and a source without an id has no constraints: its
    /// session is stopped at once.
    fn toplevel_source_created(
        &mut self,
        source: ImageCaptureSource,
        toplevel: ForeignToplevelHandle,
    ) {
        let Some(window) = window_of_identifier(&toplevel.identifier()) else {
            return;
        };
        if !self.wl.foreign_toplevel_list.handles.contains_key(&window) {
            return;
        }
        let id = capture_source::on_toplevel_source(&mut self.wl.capture_source.book, window);
        source.user_data().insert_if_missing_threadsafe(|| id);
    }
}

/// The size of `window`'s content in logical pixels and the scale of the output it is on:
/// what a capture is sized from. `None` when the window is not drawn or has no output.
pub(crate) fn content_and_scale(state: &State, window: WindowId) -> Option<(LogicalSize, Scale)> {
    let model = &state.core.model;
    let held = model.windows().get(&window)?;
    let content = state.render.textures.window(&window)?.committed;
    let output = match &held.place {
        SpacePlace::On(id) => model.spaces().get(id).map(|space| space.output.clone()),
        SpacePlace::AllSpaces => held.home.clone(),
    }?;
    let scale = model.outputs().0.get(&output)?.config.scale;
    Some((content, scale))
}

/// What a client must allocate to capture `window` now: its content size at the scale of the
/// output it is on, in the two shared-memory formats every client has or the dma-buf formats the
/// renderer draws into.
fn window_constraints(state: &State, window: WindowId) -> Option<BufferConstraints> {
    let (content, scale) = content_and_scale(state, window)?;
    let size = window_capture_size(content, scale);
    let side = |n: u32| i32::try_from(n).ok().filter(|n| *n > 0);
    Some(BufferConstraints {
        size: Size::from((side(size.w)?, side(size.h)?)),
        shm: vec![Format::Argb8888, Format::Xrgb8888],
        dma: super::image_capture::dma_offer_of(state),
    })
}

/// What a client must allocate to capture the window `source` shows now (`capture_size`).
/// `None` when the window is gone: the session is stopped.
pub(super) fn constraints(state: &State, source: ToplevelSourceId) -> Option<BufferConstraints> {
    let window = *state.wl.capture_source.book.sources.get(&source)?;
    window_constraints(state, window)
}

/// A session started on `source`: it is noted in the book, given its id in its user data and
/// held until its client destroys it or its window closes.
pub(super) fn new_session(state: &mut State, session: Session, source: ToplevelSourceId) {
    match capture_source::on_session(&mut state.wl.capture_source.book, source) {
        Ok(id) => {
            session.user_data().insert_if_missing_threadsafe(|| id);
            state.wl.capture_source.hold(id, session);
        }
        // Dropping the session tells its client `stopped`.
        Err(fault) => tracing::warn!(%fault, "a capture session on an unknown window source"),
    }
}

/// Answers one frame request of `id`: the damage gate holds it (nothing is sent) or the
/// window is rendered into the client's buffer and the frame is `ready`, or `failed`.
pub(super) fn serve(state: &mut State, session: &SessionRef, frame: Frame, id: ToplevelSessionId) {
    let Some(window) = state.wl.capture_source.book.sessions.get(&id).copied() else {
        return frame.fail(CaptureFailure::OutputGone.reason());
    };
    let Some(wanted) = window_constraints(state, window) else {
        // The window cannot be drawn (it closed, or has no output): the session is over.
        let _ = capture_source::apply_failed(
            &mut state.wl.capture_source.book,
            id,
            CaptureFailure::OutputGone,
        );
        state.wl.capture_source.stop(&[id]);
        return frame.fail(CaptureFailure::OutputGone.reason());
    };
    // The window or its output's scale changed since the session announced its constraints:
    // announce the new ones and fail this frame (the client allocates again, and its next
    // frame is drawn even if the content is the same).
    if session.current_constraints().map(|c| c.size) != Some(wanted.size) {
        tracing::debug!(size = ?wanted.size, "a window capture session changed size");
        session.update_constraints(wanted);
        state.wl.capture_source.book.forget_delivered(id);
        return fail(state, id, frame, CaptureFailure::BufferMismatch);
    }
    let Some(size) = super::image_capture::buffer_size(&frame.buffer()) else {
        return fail(state, id, frame, CaptureFailure::BufferMismatch);
    };
    if (i64::from(size.w), i64::from(size.h))
        != (i64::from(wanted.size.w), i64::from(wanted.size.h))
    {
        return fail(state, id, frame, CaptureFailure::BufferMismatch);
    }
    let book = &mut state.wl.capture_source.book;
    let content = book.content_of(window);
    match capture_source::on_frame_request(book, id, size, content) {
        Ok(FrameVerdict::Capture(request)) => answer(state, &request, frame),
        Ok(FrameVerdict::Hold) => state.wl.capture_source.hold_frame(id, frame),
        Ok(_) => fail(state, id, frame, CaptureFailure::Renderer),
        Err(fault) => {
            tracing::debug!(%fault, "a frame on a window capture session that is gone");
            frame.fail(CaptureFailure::OutputGone.reason());
        }
    }
}

/// The client destroyed the session `id`.
pub(super) fn release(state: &mut State, id: ToplevelSessionId) {
    state.wl.capture_source.release(id);
    state.wl.image_capture.copy.cleanup();
}

/// A surface committed: when it belongs to a captured window, the window's content moved on.
/// The sessions whose window changed size are told the new constraints (and the frame they hold
/// is failed so the client allocates again); the held requests the change is news to are drawn.
pub(super) fn committed(state: &mut State, surface: &WlSurface) {
    let root = casement_wayland::compositor::root_of(surface);
    let Some(window) = state.wl.xdg_shell.window_of_surface(&root) else {
        return;
    };
    super::space_shot::committed(state, window);
    let sessions = state.wl.capture_source.book.sessions_of(window);
    if sessions.is_empty() {
        return;
    }
    let wanted = window_constraints(state, window);
    for id in sessions {
        let Some(wanted) = wanted.clone() else {
            continue;
        };
        let current = state
            .wl
            .capture_source
            .session(id)
            .and_then(|held| held.current_constraints());
        if current.map(|c| c.size) == Some(wanted.size) {
            continue;
        }
        if let Some(held) = state.wl.capture_source.session(id) {
            held.update_constraints(wanted);
        }
        state.wl.capture_source.book.forget_delivered(id);
        if let Some(frame) = state.wl.capture_source.take_frame(id) {
            fail(state, id, frame, CaptureFailure::BufferMismatch);
        }
    }
    let released = capture_source::on_commit(&mut state.wl.capture_source.book, window);
    for request in released {
        if let Some(frame) = state.wl.capture_source.take_frame(request.session) {
            answer(state, &request, frame);
        }
    }
}

/// Draws the window of `request` into `frame`'s buffer and tells the client `ready`, or
/// `failed`.
fn answer(state: &mut State, request: &ToplevelCaptureRequest, frame: Frame) {
    let id = request.session;
    // A frame the client gave up on (or that failed on its own) is not drawn.
    if frame.has_failed() {
        let _ = capture_source::apply_failed(
            &mut state.wl.capture_source.book,
            id,
            CaptureFailure::Renderer,
        );
        return;
    }
    match draw_window(state, request, &frame.buffer()) {
        Ok(at) => {
            let ready = capture_source::apply_ready(
                &mut state.wl.capture_source.book,
                id,
                request.content,
                at,
            );
            if let Err(fault) = ready {
                tracing::debug!(%fault, "a window capture frame was not pending");
            }
            let size = Size::from((
                i32::try_from(request.size.w).unwrap_or(i32::MAX),
                i32::try_from(request.size.h).unwrap_or(i32::MAX),
            ));
            frame.success(
                Transform::Normal,
                vec![Rectangle::<i32, Buffer>::from_size(size)],
                at.0,
            );
        }
        Err(why) => fail(state, id, frame, why),
    }
}

/// Fails `frame` with `why` and clears what the book noted of it.
fn fail(state: &mut State, id: ToplevelSessionId, frame: Frame, why: CaptureFailure) {
    if let Err(fault) = capture_source::apply_failed(&mut state.wl.capture_source.book, id, why) {
        tracing::debug!(%fault, "a window capture frame was not pending");
    }
    frame.fail(why.reason());
}
