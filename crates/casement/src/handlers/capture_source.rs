//! Toplevel image capture sources (P2-Z; filled by P2-Q3): the session half of
//! `ext_foreign_toplevel_image_capture_source_manager_v1`. The sources and sessions are the
//! ext-image-copy-capture objects `handlers/image_capture.rs` already serves; that file hands
//! the ones that belong to a window here (a source whose user data holds a `ToplevelSourceId`,
//! a session whose user data holds a `ToplevelSessionId`). P2-Q3 also implements smithay's
//! `ToplevelCaptureSourceHandler` for the state in this file, once the part holds its state.

use crate::state::State;
use casement_wayland::capture_source::{ToplevelSessionId, ToplevelSourceId};
use smithay::wayland::image_copy_capture::{BufferConstraints, Frame, Session, SessionRef};

/// What a client must allocate to capture the window `source` shows now: its content size at
/// the scale of the output it is on (`capture_size`), in the two shared-memory formats every
/// client has. `None` when the window is gone.
pub(super) fn constraints(state: &State, source: ToplevelSourceId) -> Option<BufferConstraints> {
    let _ = (state, source);
    todo!("P2-Q3: constraints of a toplevel source")
}

/// A session started on `source`: it is noted in the book, given its id in its user data and
/// held until its client destroys it or its window closes.
pub(super) fn new_session(state: &mut State, session: Session, source: ToplevelSourceId) {
    let _ = (state, session, source);
    todo!("P2-Q3: a session on a toplevel source")
}

/// Answers one frame request of `id`: the damage gate holds it (nothing is sent) or the
/// window is rendered into the client's buffer and the frame is `ready`, or `failed`.
pub(super) fn serve(state: &mut State, session: &SessionRef, frame: Frame, id: ToplevelSessionId) {
    let _ = (state, session, frame, id);
    todo!("P2-Q3: a frame of a toplevel session")
}

/// The client destroyed the session `id`.
pub(super) fn release(state: &mut State, id: ToplevelSessionId) {
    let _ = (state, id);
    todo!("P2-Q3: release a toplevel session")
}
