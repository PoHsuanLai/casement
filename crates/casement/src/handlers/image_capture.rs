//! `ext_output_image_capture_source_manager_v1` and `ext_image_copy_capture_manager_v1`
//! (P1-C1): the smithay handler traits for image capture sources and copy sessions.
//!
//! A frame request is answered at once from the frame the output last presented, the one
//! scanout shows, so a capture equals the scanout byte for byte. On the headless backend that
//! frame was already read back when it was presented (through the same `ExportMem` pass
//! `casement_render::pass::capture_output` uses). On a tty seat, while a capture session is open
//! on an output, each frame is also drawn through pixman into a shadow (P1-T,
//! `casement_backend::tty::Readback`); the first frame of a session fails until the next vblank
//! has drawn one (FINDINGS, p1-t).

mod cursor;
mod dma;
mod frames;

pub(super) use cursor::sync as sync_cursors;
pub(super) use dma::{buffer_size, deliver, fourccs, offer as dma_offer_of};
pub(super) use frames::{last_frame, note_presented, release_waiting};

use super::{capture_source, space_shot};
use crate::state::State;
use casement_core::OutputKey;
use casement_wayland::capture_source::{ToplevelSessionId, ToplevelSourceId};
use casement_wayland::image_capture::{self, CaptureSessionId, CaptureSourceId};
use casement_wayland::quire::space_shot::{SpaceSessionId, SpaceSourceId};
use smithay::output::Output;
use smithay::reexports::wayland_server::protocol::wl_pointer::WlPointer;
use smithay::reexports::wayland_server::protocol::wl_shm::Format;
use smithay::utils::Size;
use smithay::wayland::image_capture_source::{
    ImageCaptureSource, ImageCaptureSourceHandler, OutputCaptureSourceHandler,
    OutputCaptureSourceState,
};
use smithay::wayland::image_copy_capture::{
    BufferConstraints, CursorSession, CursorSessionRef, Frame, ImageCopyCaptureHandler,
    ImageCopyCaptureState, Session, SessionRef,
};

impl ImageCaptureSourceHandler for State {
    fn source_destroyed(&mut self, source: ImageCaptureSource) {
        if let Some(id) = source.user_data().get::<CaptureSourceId>() {
            image_capture::on_source_destroyed(&mut self.wl.image_capture.book, *id);
        }
        if let Some(id) = source.user_data().get::<SpaceSourceId>() {
            self.wl.quire_shell.shots.book.on_source_destroyed(*id);
        }
        // A window source (P2-Q3) leaves the window book with it, not when its window closes.
        if let Some(id) = source
            .user_data()
            .get::<casement_wayland::capture_source::ToplevelSourceId>()
        {
            casement_wayland::capture_source::on_source_destroyed(
                &mut self.wl.capture_source.book,
                *id,
            );
        }
    }
}

impl OutputCaptureSourceHandler for State {
    fn output_capture_source_state(&mut self) -> &mut OutputCaptureSourceState {
        &mut self.wl.image_capture.output_sources
    }

    fn output_source_created(&mut self, source: ImageCaptureSource, output: &Output) {
        let key = self
            .wl
            .output
            .outputs
            .iter()
            .find(|(_, advertised)| *advertised == output)
            .map(|(key, _)| key.clone());
        if let Some(key) = key {
            let id = image_capture::on_output_source(&mut self.wl.image_capture.book, key);
            source.user_data().insert_if_missing_threadsafe(|| id);
        }
    }
}

impl ImageCopyCaptureHandler for State {
    fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState {
        &mut self.wl.image_capture.copy
    }

    fn capture_constraints(&mut self, source: &ImageCaptureSource) -> Option<BufferConstraints> {
        if let Some(window) = source.user_data().get::<ToplevelSourceId>().copied() {
            return capture_source::constraints(self, window);
        }
        if let Some(shot) = source.user_data().get::<SpaceSourceId>().copied() {
            return space_shot::constraints(self, shot);
        }
        let id = source.user_data().get::<CaptureSourceId>()?;
        let output = self.wl.image_capture.book.sources.get(id)?.clone();
        constraints_of(self, &output)
    }

    fn cursor_capture_constraints(
        &mut self,
        source: &ImageCaptureSource,
        _pointer: &WlPointer,
    ) -> Option<BufferConstraints> {
        cursor::constraints(self, source)
    }

    fn new_cursor_session(&mut self, session: CursorSession) {
        cursor::new_session(self, session);
    }

    fn cursor_frame(&mut self, session: &CursorSessionRef, frame: Frame) {
        cursor::serve(self, session, frame);
    }

    fn cursor_session_destroyed(&mut self, session: CursorSessionRef) {
        cursor::release(self, &session);
    }

    fn new_session(&mut self, session: Session) {
        let source = session.source();
        if let Some(window) = source.user_data().get::<ToplevelSourceId>().copied() {
            return capture_source::new_session(self, session, window);
        }
        if let Some(shot) = source.user_data().get::<SpaceSourceId>().copied() {
            return space_shot::new_session(self, session, shot);
        }
        let Some(source) = source.user_data().get::<CaptureSourceId>().copied() else {
            return;
        };
        match image_capture::on_session(&mut self.wl.image_capture.book, source) {
            Ok(id) => {
                session.user_data().insert_if_missing_threadsafe(|| id);
                self.wl.image_capture.hold(id, session);
            }
            // Dropping the session tells its client `stopped`.
            Err(fault) => tracing::warn!(%fault, "a capture session on an unknown source"),
        }
    }

    fn frame(&mut self, session: &SessionRef, frame: Frame) {
        if let Some(id) = session.user_data().get::<ToplevelSessionId>().copied() {
            return capture_source::serve(self, session, frame, id);
        }
        if let Some(id) = session.user_data().get::<SpaceSessionId>().copied() {
            return space_shot::serve(self, session, frame, id);
        }
        frames::serve(self, session, frame);
    }

    fn session_destroyed(&mut self, session: SessionRef) {
        if let Some(id) = session.user_data().get::<ToplevelSessionId>().copied() {
            return capture_source::release(self, id);
        }
        if let Some(id) = session.user_data().get::<SpaceSessionId>().copied() {
            return space_shot::release(self, id);
        }
        if let Some(id) = session.user_data().get::<CaptureSessionId>() {
            self.wl.image_capture.release(*id);
        }
    }
}

/// What a client must allocate to capture `output` now: its mode's size, in the two shared
/// memory formats every client has or, when the renderer can draw into one, a dma-buf of the
/// formats `dma::offer` names (shared memory stays the fallback).
pub(super) fn constraints_of(state: &State, output: &OutputKey) -> Option<BufferConstraints> {
    let mode = state.core.model.outputs().0.get(output)?.config.mode.size;
    let side = |n: u32| i32::try_from(n).ok();
    Some(BufferConstraints {
        size: Size::from((side(mode.w)?, side(mode.h)?)),
        shm: vec![Format::Argb8888, Format::Xrgb8888],
        dma: dma::offer(state),
    })
}
