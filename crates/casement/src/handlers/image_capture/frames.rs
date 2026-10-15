//! Answering one frame request.

use super::{buffer_size, constraints_of, deliver};
use crate::state::State;
use casement_backend::{BackendKind, FramePixels, PresentedFrame};
use casement_core::OutputKey;
use casement_core::geometry::PhysicalRect;
use casement_core::time::Mono;
use casement_render::pass::CapturedFrame;
use casement_wayland::image_capture::{self, CaptureFailure, CaptureSessionId, CursorPaint};
use smithay::utils::{Buffer, Rectangle, Size, Transform};
use smithay::wayland::image_copy_capture::{Frame, SessionRef};

/// Fills the client's buffer from the output's last frame and tells it `ready`, or `failed`
/// with the reason.
pub(super) fn serve(state: &mut State, session: &SessionRef, frame: Frame) {
    let Some(id) = session.user_data().get::<CaptureSessionId>().copied() else {
        return frame.fail(CaptureFailure::OutputGone.reason());
    };
    match capture(state, session, id, &frame) {
        // No frame has been presented for the output yet (a first capture on a real seat waits
        // for the shadow's first draw): keep the request until `release_waiting`.
        Ok(None) => state.wl.image_capture.wait_frame(id, frame),
        Ok(Some((at, damage))) => {
            if let Err(fault) = image_capture::apply_ready(&mut state.wl.image_capture.book, id, at)
            {
                tracing::debug!(%fault, "a capture frame was not pending");
            }
            frame.success(Transform::Normal, damage, at.0);
        }
        Err(why) => {
            // A frame refused before it was noted (a buffer of no size) has nothing to clear.
            if let Err(fault) =
                image_capture::apply_failed(&mut state.wl.image_capture.book, id, why)
            {
                tracing::debug!(%fault, "a capture frame was not pending");
            }
            frame.fail(why.reason());
        }
    }
}

/// A frame written into a client's buffer: when it was presented and what changed since the
/// session's last frame.
type Written = (Mono, Vec<Rectangle<i32, Buffer>>);

/// The frame written into `frame`'s buffer: when it was presented and what changed since the
/// session's last frame. None while there is nothing to send: no frame has been presented, or
/// (after the session's first frame) nothing changed.
fn capture(
    state: &mut State,
    session: &SessionRef,
    id: CaptureSessionId,
    frame: &Frame,
) -> Result<Option<Written>, CaptureFailure> {
    let buffer = frame.buffer();
    let size = buffer_size(&buffer).ok_or(CaptureFailure::BufferMismatch)?;
    let request = image_capture::on_frame_request(&mut state.wl.image_capture.book, id, size)
        .map_err(|_| CaptureFailure::OutputGone)?;
    let wanted = constraints_of(state, &request.output).ok_or(CaptureFailure::OutputGone)?;
    // The mode changed since the session announced its constraints: announce the new ones and
    // fail this frame (the client allocates again).
    if session.current_constraints().map(|c| c.size) != Some(wanted.size) {
        tracing::debug!(size = ?wanted.size, "a capture session's output changed size");
        session.update_constraints(wanted);
        return Err(CaptureFailure::BufferMismatch);
    }
    let paint = match session.draw_cursor() {
        true => CursorPaint::Painted,
        false => CursorPaint::Hidden,
    };
    let Some((captured, at)) = last_frame(state, &request.output, paint)? else {
        return Ok(None);
    };
    let last = state.wl.image_capture.book.delivered.get(&id).copied();
    let changed = state
        .wl
        .image_capture
        .damage
        .since(&request.output, last)
        .within(captured.area);
    // The first frame of a session is whole; the next waits for something to change.
    if last.is_some() && changed.is_empty() {
        return Ok(None);
    }
    deliver(state, &buffer, &captured).map_err(|error| {
        tracing::warn!(%error, "a capture frame was not written");
        CaptureFailure::BufferMismatch
    })?;
    Ok(Some((
        at,
        changed.into_iter().filter_map(buffer_rect).collect(),
    )))
}

/// `rect` as smithay's buffer rectangle.
fn buffer_rect(rect: PhysicalRect) -> Option<Rectangle<i32, Buffer>> {
    let side = |n: u32| i32::try_from(n).ok();
    Some(Rectangle::new(
        (rect.x, rect.y).into(),
        Size::from((side(rect.w)?, side(rect.h)?)),
    ))
}

/// The frame `output` last presented, with or without the cursor as `paint` says, as a capture,
/// and its instant; none while the output has presented none (or, on a real seat, the shadow
/// has drawn none).
pub(in crate::handlers) fn last_frame(
    state: &State,
    output: &OutputKey,
    paint: CursorPaint,
) -> Result<Option<(CapturedFrame, Mono)>, CaptureFailure> {
    let Some(presented) = presented(state, output)? else {
        return Ok(None);
    };
    let FramePixels::Rgba(shown) = &presented.pixels else {
        return Err(CaptureFailure::Renderer);
    };
    // A frame without the cursor is kept beside the one shown; if the renderer made none, the
    // one shown is the best there is.
    let pixels = match (paint, presented.bare.as_ref()) {
        (CursorPaint::Hidden, Some(bare)) => bare,
        _ => shown,
    };
    let area = PhysicalRect {
        x: 0,
        y: 0,
        w: pixels.size.w,
        h: pixels.size.h,
    };
    Ok(Some((
        CapturedFrame::from_rgba(area, &pixels.bytes),
        presented.at,
    )))
}

/// The frame `output` last presented, as its backend kept it.
fn presented<'a>(
    state: &'a State,
    output: &OutputKey,
) -> Result<Option<&'a PresentedFrame>, CaptureFailure> {
    let name = state
        .core
        .model
        .outputs()
        .0
        .get(output)
        .map(|plugged| plugged.facts.name.clone())
        .ok_or(CaptureFailure::OutputGone)?;
    // The headless backend kept the frame it presented; a tty seat kept the shadow it drew
    // while this output was watched (the first frame of a session waits for the next vblank).
    match &state.backend {
        BackendKind::Headless(backend) => Ok(backend.last_frame(&name)),
        BackendKind::Tty(tty) => Ok(tty.last_frame(&name)),
        _ => Err(CaptureFailure::Renderer),
    }
}

/// Notes in the damage log what the frame `output` last presented changed. Run after every
/// frame the loop presents; a frame already noted is ignored.
pub(in crate::handlers) fn note_presented(state: &mut State, output: &OutputKey) {
    let Ok(Some(frame)) = presented(state, output) else {
        return;
    };
    let (at, damage) = (frame.at, frame.damage.clone());
    state.wl.image_capture.damage.record(output, at, damage);
}

/// Serves again the frame requests that waited for a first frame, now that one may exist.
pub(in crate::handlers) fn release_waiting(state: &mut State) {
    for (id, frame) in state.wl.image_capture.take_waiting() {
        let Some(session) = state.wl.image_capture.session_ref(id) else {
            continue;
        };
        serve(state, &session, frame);
    }
}
