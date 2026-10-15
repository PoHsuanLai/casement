//! Space pictures (`quire_shell_v1.get_space_capture_source`): the session half of the
//! ext-image-copy-capture objects that show one Space of an output as a small picture for the
//! shell's Spaces bar. `handlers/image_capture.rs` hands the sessions whose source holds a
//! `SpaceSourceId` here.
//!
//! A picture is the windows of the Space the output shows, each drawn alone (window capture's
//! pixels, no frames) and shrunk to where it sits on the output ([`compose_thumb`]), over a
//! transparent background. A frame request is held until the Space changed since the session's
//! last frame: a window of it committed, moved or came or went ([`look`]). This version draws
//! the Space the output shows now; a session on another Space is stopped.

use super::capture_source::window_rgba;
use crate::state::State;
use casement_core::geometry::{PhysicalRect, PhysicalSize};
use casement_render::pass::{CapturedFrame, ThumbLayer, compose_thumb, thumb_dest};
use casement_wayland::image_capture::CaptureFailure;
use casement_wayland::quire::space_shot::{
    Look, SpaceSessionId, SpaceShot, SpaceSourceId, Verdict, look_of,
};
use smithay::reexports::wayland_server::protocol::wl_shm::Format;
use smithay::utils::{Buffer, Rectangle, Size, Transform};
use smithay::wayland::image_copy_capture::{BufferConstraints, Frame, Session, SessionRef};

/// Whether the Space of `shot` is the one its output shows now.
fn is_shown(state: &State, shot: &SpaceShot) -> bool {
    let spaces = state.core.model.spaces();
    spaces
        .shown_on(&shot.output)
        .is_some_and(|shown| shown.id == shot.space)
}

/// The windows of the Space of `shot` that are drawn: their rectangles, in global logical
/// pixels, back first.
fn windows_of(
    state: &State,
    shot: &SpaceShot,
) -> Vec<(
    casement_core::WindowId,
    casement_core::geometry::LogicalRect,
)> {
    state
        .core
        .arrangement
        .windows
        .iter()
        .filter(|target| target.output == shot.output)
        .filter(|target| state.core.model.shows(target.window))
        .map(|target| (target.window, target.rect))
        .collect()
}

/// The fingerprint of what the Space of `shot` shows now.
pub(crate) fn look(state: &State, shot: &SpaceShot) -> Look {
    let book = &state.wl.quire_shell.shots;
    look_of(
        windows_of(state, shot)
            .into_iter()
            .flat_map(|(window, rect)| {
                [
                    i64::try_from(window.0).unwrap_or(i64::MAX),
                    i64::from(rect.origin.x),
                    i64::from(rect.origin.y),
                    i64::from(rect.size.w),
                    i64::from(rect.size.h),
                    i64::try_from(book.commits_of(window)).unwrap_or(i64::MAX),
                ]
            }),
    )
}

/// What a client must allocate to capture the Space `source` shows: the size it asked for. None
/// when the Space is gone or not the one shown: the session is stopped.
pub(super) fn constraints(state: &State, source: SpaceSourceId) -> Option<BufferConstraints> {
    let shot = state.wl.quire_shell.shots.book.source(source)?;
    if !is_shown(state, shot) {
        return None;
    }
    constraints_of(state, shot)
}

fn constraints_of(state: &State, shot: &SpaceShot) -> Option<BufferConstraints> {
    let side = |n: u32| i32::try_from(n).ok().filter(|n| *n > 0);
    Some(BufferConstraints {
        size: Size::from((side(shot.size.w)?, side(shot.size.h)?)),
        shm: vec![Format::Argb8888, Format::Xrgb8888],
        dma: super::image_capture::dma_offer_of(state),
    })
}

/// A session started on `source`: noted in the book, given its id and held until its client
/// destroys it or its Space stops showing.
pub(super) fn new_session(state: &mut State, session: Session, source: SpaceSourceId) {
    let shots = &mut state.wl.quire_shell.shots;
    match shots.book.on_session(source) {
        Some(id) => {
            session.user_data().insert_if_missing_threadsafe(|| id);
            shots.hold(id, session);
        }
        // Dropping the session tells its client `stopped`.
        None => tracing::warn!("a capture session on a Space source that is gone"),
    }
}

/// Answers one frame request of `id`: held until the Space changes, or drawn now, or failed.
pub(super) fn serve(state: &mut State, session: &SessionRef, frame: Frame, id: SpaceSessionId) {
    let Some(shot) = state.wl.quire_shell.shots.book.session(id).cloned() else {
        return frame.fail(CaptureFailure::OutputGone.reason());
    };
    if !is_shown(state, &shot) {
        state.wl.quire_shell.shots.stop(&[id]);
        return frame.fail(CaptureFailure::OutputGone.reason());
    }
    let sized = super::image_capture::buffer_size(&frame.buffer());
    if sized != Some(shot.size) {
        // The buffer does not fit what the session announced: say what fits and fail the frame.
        if let Some(wanted) = constraints_of(state, &shot) {
            session.update_constraints(wanted);
        }
        return frame.fail(CaptureFailure::BufferMismatch.reason());
    }
    let now = look(state, &shot);
    match state.wl.quire_shell.shots.book.request(id, now) {
        Some(Verdict::Draw) => answer(state, id, &shot, now, frame),
        Some(Verdict::Hold) => state.wl.quire_shell.shots.hold_frame(id, frame),
        Some(_) | None => frame.fail(CaptureFailure::Renderer.reason()),
    }
}

/// The client destroyed the session `id`.
pub(super) fn release(state: &mut State, id: SpaceSessionId) {
    state.wl.quire_shell.shots.release(id);
    state.wl.image_capture.copy.cleanup();
}

/// A window committed: its count moves, and a held request that this is news to is drawn.
pub(crate) fn committed(state: &mut State, window: casement_core::WindowId) {
    state.wl.quire_shell.shots.note_commit(window);
    poll(state);
}

/// Draws the held requests whose Space changed since their session's last frame, and stops the
/// sessions whose Space is no longer shown. Run after a commit and after a relayout.
pub(crate) fn poll(state: &mut State) {
    if !state.wl.quire_shell.shots.book.is_watching() {
        return;
    }
    let ids = state.wl.quire_shell.shots.book.sessions();
    let shots: Vec<(SpaceSessionId, SpaceShot)> = ids
        .into_iter()
        .filter_map(|id| {
            let shot = state.wl.quire_shell.shots.book.session(id)?.clone();
            Some((id, shot))
        })
        .collect();
    let gone: Vec<SpaceSessionId> = shots
        .iter()
        .filter(|(_, shot)| !is_shown(state, shot))
        .map(|(id, _)| *id)
        .collect();
    state.wl.quire_shell.shots.stop(&gone);
    let looks: Vec<(SpaceShot, Look)> = shots
        .iter()
        .filter(|(id, _)| !gone.contains(id))
        .map(|(_, shot)| (shot.clone(), look(state, shot)))
        .collect();
    let due = state.wl.quire_shell.shots.book.changed(|shot| {
        looks
            .iter()
            .find(|(seen, _)| seen == shot)
            .map_or_else(Look::default, |(_, look)| *look)
    });
    for id in due {
        let (Some(shot), Some(frame)) = (
            shots
                .iter()
                .find(|(s, _)| *s == id)
                .map(|(_, shot)| shot.clone()),
            state.wl.quire_shell.shots.take_frame(id),
        ) else {
            continue;
        };
        let now = look(state, &shot);
        answer(state, id, &shot, now, frame);
    }
}

/// Draws the Space into `frame`'s buffer and tells the client `ready`, or `failed`.
fn answer(state: &mut State, id: SpaceSessionId, shot: &SpaceShot, now: Look, frame: Frame) {
    if frame.has_failed() {
        return;
    }
    match picture(state, shot).and_then(|bytes| write(state, shot.size, &bytes, &frame)) {
        Ok(()) => {
            state.wl.quire_shell.shots.book.delivered(id, now);
            let at = state.clock.now();
            let size = Size::from((
                i32::try_from(shot.size.w).unwrap_or(i32::MAX),
                i32::try_from(shot.size.h).unwrap_or(i32::MAX),
            ));
            frame.success(
                Transform::Normal,
                vec![Rectangle::<i32, Buffer>::from_size(size)],
                at.0,
            );
        }
        Err(why) => {
            state.wl.quire_shell.shots.book.forget(id);
            frame.fail(why.reason());
        }
    }
}

/// The picture of the Space: its windows drawn alone, shrunk into place.
fn picture(state: &mut State, shot: &SpaceShot) -> Result<Vec<u8>, CaptureFailure> {
    let output = state
        .core
        .model
        .outputs()
        .0
        .get(&shot.output)
        .map(|plugged| plugged.config.logical_rect())
        .ok_or(CaptureFailure::OutputGone)?;
    let mut drawn = Vec::new();
    for (window, rect) in windows_of(state, shot) {
        // A window that cannot be drawn (no buffer yet) is left out of the picture.
        if let Ok(pixels) = window_rgba(state, window) {
            drawn.push((thumb_dest(rect, output, shot.size), pixels));
        }
    }
    let layers: Vec<ThumbLayer<'_>> = drawn
        .iter()
        .map(|(dest, pixels)| ThumbLayer::new(*dest, pixels.size, &pixels.bytes))
        .collect();
    Ok(compose_thumb(shot.size, &layers))
}

/// Puts the picture into the client's buffer.
fn write(
    state: &mut State,
    size: PhysicalSize,
    rgba: &[u8],
    frame: &Frame,
) -> Result<(), CaptureFailure> {
    let area = PhysicalRect {
        x: 0,
        y: 0,
        w: size.w,
        h: size.h,
    };
    let captured = CapturedFrame::from_rgba(area, rgba);
    super::image_capture::deliver(state, &frame.buffer(), &captured).map_err(|error| {
        tracing::warn!(%error, "a Space picture was not written");
        CaptureFailure::BufferMismatch
    })
}
