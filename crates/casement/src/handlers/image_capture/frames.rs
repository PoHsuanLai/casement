//! Answering one frame request.

use super::constraints_of;
use crate::state::State;
use casement_backend::{BackendKind, FramePixels};
use casement_core::OutputKey;
use casement_core::geometry::{PhysicalRect, PhysicalSize};
use casement_core::time::Mono;
use casement_render::pass::{CapturedFrame, write_frame};
use casement_wayland::image_capture::{self, CaptureFailure, CaptureSessionId};
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::utils::{Buffer, Rectangle, Size, Transform};
use smithay::wayland::image_copy_capture::{Frame, SessionRef};
use smithay::wayland::shm::with_buffer_contents;

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
        Ok(Some((at, size))) => {
            if let Err(fault) = image_capture::apply_ready(&mut state.wl.image_capture.book, id, at)
            {
                tracing::debug!(%fault, "a capture frame was not pending");
            }
            let full = Rectangle::<i32, Buffer>::from_size(size);
            frame.success(Transform::Normal, vec![full], at.0);
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

/// A frame written into a client's buffer: when it was presented and its size.
type Written = (Mono, Size<i32, Buffer>);

/// The frame written into `frame`'s buffer: when it was presented and its size.
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
    let Some((captured, at)) = last_frame(state, &request.output)? else {
        return Ok(None);
    };
    write_frame(&buffer, &captured).map_err(|error| {
        tracing::warn!(%error, "a capture frame was not written");
        CaptureFailure::BufferMismatch
    })?;
    let area = captured.area;
    let size = Size::from((
        i32::try_from(area.w).unwrap_or(i32::MAX),
        i32::try_from(area.h).unwrap_or(i32::MAX),
    ));
    Ok(Some((at, size)))
}

/// The frame `output` last presented, as a capture, and its instant; none while the output
/// has presented none (or, on a real seat, the shadow has drawn none).
pub(in crate::handlers) fn last_frame(
    state: &State,
    output: &OutputKey,
) -> Result<Option<(CapturedFrame, Mono)>, CaptureFailure> {
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
    let presented = match &state.backend {
        BackendKind::Headless(backend) => backend.last_frame(&name),
        BackendKind::Tty(tty) => tty.last_frame(&name),
        _ => return Err(CaptureFailure::Renderer),
    };
    let Some(presented) = presented else {
        return Ok(None);
    };
    let FramePixels::Rgba(pixels) = &presented.pixels else {
        return Err(CaptureFailure::Renderer);
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

/// Serves again the frame requests that waited for a first frame, now that one may exist.
pub(in crate::handlers) fn release_waiting(state: &mut State) {
    for (id, frame) in state.wl.image_capture.take_waiting() {
        let Some(session) = state.wl.image_capture.session_ref(id) else {
            continue;
        };
        serve(state, &session, frame);
    }
}

/// The size of a shared-memory buffer, in pixels.
fn buffer_size(buffer: &WlBuffer) -> Option<PhysicalSize> {
    let (w, h) = with_buffer_contents(buffer, |_, _, data| (data.width, data.height)).ok()?;
    Some(PhysicalSize {
        w: u32::try_from(w).ok()?,
        h: u32::try_from(h).ok()?,
    })
}
