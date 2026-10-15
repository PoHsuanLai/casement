//! `ext_image_copy_capture_cursor_session_v1`: where the pointer is on an output and, as a
//! capture source of its own, what it looks like. smithay serves the wire; the pure placement
//! is `casement_wayland::image_capture::place`.
//!
//! A cursor session is told, after every frame the output presents, whether the cursor's
//! image meets the frame (`enter`, `leave`), where its hotspot is and how far the hotspot is
//! from the image's corner. A frame request is answered with the cursor's pixels once, and
//! again when they change. Only the compositor's own cursor shapes have pixels: a cursor a
//! client drew itself is placed and sized but its frame fails (FINDINGS, wave-2026-10-15/capture2).

use crate::state::State;
use casement_core::OutputKey;
use casement_core::geometry::PhysicalRect;
use casement_render::pass::{CapturedFrame, write_frame};
use casement_render::{ChromeLayer, CursorDraw, CursorShape, cursor_face, cursor_pixels};
use casement_wayland::image_capture::{
    CaptureFailure, CaptureSourceId, CursorSessionId, CursorView, image_size, place,
};
use smithay::reexports::wayland_server::protocol::wl_shm::Format;
use smithay::utils::{Buffer, Rectangle, Size, Transform};
use smithay::wayland::image_capture_source::ImageCaptureSource;
use smithay::wayland::image_copy_capture::{
    BufferConstraints, CursorSession, CursorSessionRef, Frame,
};

/// The output a source shows.
fn output_of(state: &State, source: &ImageCaptureSource) -> Option<OutputKey> {
    let id = source.user_data().get::<CaptureSourceId>()?;
    state.wl.image_capture.book.sources.get(id).cloned()
}

/// What a cursor session on `output` must allocate: the cursor's image at the output's scale
/// (the arrow's while no cursor is shown), in the two shared-memory formats every client has.
fn constraints_for(state: &State, output: &OutputKey) -> Option<BufferConstraints> {
    let config = state.core.model.outputs().0.get(output)?.config;
    let arrow = ChromeLayer {
        cursor: CursorDraw::Named(CursorShape::Default),
        cursor_at: state.render.chrome.cursor_at,
        covers: Vec::new(),
    };
    let book = &state.render.textures;
    let face = cursor_face(&state.render.chrome, book).or_else(|| cursor_face(&arrow, book))?;
    let size = image_size(face.size, config.scale);
    Some(BufferConstraints {
        size: Size::from((i32::try_from(size.w).ok()?, i32::try_from(size.h).ok()?)),
        shm: vec![Format::Argb8888, Format::Xrgb8888],
        dma: None,
    })
}

/// The constraints of a cursor session on `source`; none for a source that is not an output
/// (the session is stopped).
pub(super) fn constraints(state: &State, source: &ImageCaptureSource) -> Option<BufferConstraints> {
    let output = output_of(state, source)?;
    constraints_for(state, &output)
}

/// A client made a cursor session: it is held and told where the cursor is.
pub(super) fn new_session(state: &mut State, session: CursorSession) {
    let Some(output) = output_of(state, &session.source()) else {
        // Dropping the session ends it.
        return;
    };
    let id = state.wl.image_capture.cursors.hold(session, output.clone());
    if let Some(hold) = state.wl.image_capture.cursors.get(id) {
        hold.session.user_data().insert_if_missing_threadsafe(|| id);
    }
    sync(state, &output);
}

/// The client destroyed a cursor session.
pub(super) fn release(state: &mut State, session: &CursorSessionRef) {
    if let Some(id) = session.user_data().get::<CursorSessionId>().copied() {
        state.wl.image_capture.cursors.release(id);
    }
    state.wl.image_capture.copy.cleanup();
}

/// Tells each cursor session on `output` where the cursor is now, and serves the frame request
/// that waited for the image to change. Run after every frame the output presents.
pub(crate) fn sync(state: &mut State, output: &OutputKey) {
    let ids = state.wl.image_capture.cursors.on(output);
    let Some(config) = state.core.model.outputs().0.get(output).map(|p| p.config) else {
        return;
    };
    if ids.is_empty() {
        return;
    }
    let view = CursorView::new(
        config.logical_rect().origin,
        config.scale,
        config.mode.size,
        config.transform,
    );
    let chrome = &state.render.chrome;
    let placement = cursor_face(chrome, &state.render.textures)
        .and_then(|face| place(&view, chrome.cursor_at, face.size, face.hotspot));
    let wanted = constraints_for(state, output);
    for id in ids {
        let Some(session) = state
            .wl
            .image_capture
            .cursors
            .get(id)
            .map(|hold| hold.session.as_ref())
        else {
            continue;
        };
        if let Some(wanted) = wanted.clone()
            && session.current_constraints().map(|c| c.size) != Some(wanted.size)
        {
            session.update_constraints(wanted);
        }
        match placement {
            Some(placed) => {
                // The hotspot first: entering sends it with the position.
                session.set_cursor_hotspot(placed.hotspot);
                session.set_cursor_pos(Some(placed.position.into()));
            }
            None => session.set_cursor_pos(None),
        }
        if let Some(frame) = state.wl.image_capture.cursors.take_waiting(id) {
            serve(state, &session, frame);
        }
    }
}

/// Answers a request for the cursor's image: its pixels at the output's scale, or the request
/// waits while they are the ones the client already has.
pub(super) fn serve(state: &mut State, session: &CursorSessionRef, frame: Frame) {
    let Some(id) = session.user_data().get::<CursorSessionId>().copied() else {
        return frame.fail(CaptureFailure::OutputGone.reason());
    };
    match image(state, session, id) {
        Ok(Image::Fresh(argb, size)) => {
            let area = PhysicalRect {
                x: 0,
                y: 0,
                w: u32::try_from(size.w).unwrap_or(0),
                h: u32::try_from(size.h).unwrap_or(0),
            };
            let captured = CapturedFrame::from_argb(area, argb.clone());
            if let Err(error) = write_frame(&frame.buffer(), &captured) {
                tracing::warn!(%error, "a cursor frame was not written");
                return frame.fail(CaptureFailure::BufferMismatch.reason());
            }
            if let Some(hold) = state.wl.image_capture.cursors.get_mut(id) {
                hold.shown = Some(argb);
            }
            let at = state.clock.now();
            frame.success(
                Transform::Normal,
                vec![Rectangle::<i32, Buffer>::from_size(size)],
                at.0,
            );
        }
        Ok(Image::Same) => state.wl.image_capture.cursors.wait(id, frame),
        Err(why) => frame.fail(why.reason()),
    }
}

/// What a request for the cursor's image finds.
enum Image {
    /// Pixels the client has not got, and their size.
    Fresh(Vec<u8>, Size<i32, Buffer>),
    /// The ones it has.
    Same,
}

/// The cursor's pixels for session `id`.
fn image(
    state: &mut State,
    session: &CursorSessionRef,
    id: CursorSessionId,
) -> Result<Image, CaptureFailure> {
    let output = state
        .wl
        .image_capture
        .cursors
        .get(id)
        .map(|hold| hold.output.clone())
        .ok_or(CaptureFailure::OutputGone)?;
    let scale = state
        .core
        .model
        .outputs()
        .0
        .get(&output)
        .map(|plugged| plugged.config.scale)
        .ok_or(CaptureFailure::OutputGone)?;
    let wanted = constraints_for(state, &output).ok_or(CaptureFailure::OutputGone)?;
    // The cursor grew or the scale changed since the session announced its size: announce the
    // new one and fail this frame (the client allocates again).
    if session.current_constraints().map(|c| c.size) != Some(wanted.size) {
        session.update_constraints(wanted);
        return Err(CaptureFailure::BufferMismatch);
    }
    let argb = cursor_pixels(&state.render.chrome, scale).ok_or(CaptureFailure::Renderer)?;
    let shown = state
        .wl
        .image_capture
        .cursors
        .get(id)
        .and_then(|hold| hold.shown.as_ref());
    match shown == Some(&argb) {
        true => Ok(Image::Same),
        false => Ok(Image::Fresh(argb, wanted.size)),
    }
}
