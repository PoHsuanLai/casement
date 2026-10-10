//! `ext_output_image_capture_source_manager_v1` and `ext_image_copy_capture_manager_v1`
//! (P1-C1): the smithay handler traits for image capture sources and copy sessions.
//!
//! A frame request is answered at once from the frame the output last presented, the one
//! scanout shows, so a capture equals the scanout byte for byte. On the headless backend that
//! frame was already read back when it was presented (through the same `ExportMem` pass
//! `casement_render::pass::capture_output` uses); the tty backend's read-back arrives with
//! P1-D3's scanout and fails the frame until then (FINDINGS, p1-c1).

mod frames;

use crate::state::State;
use casement_core::OutputKey;
use casement_wayland::image_capture::{self, CaptureSessionId, CaptureSourceId};
use smithay::output::Output;
use smithay::reexports::wayland_server::protocol::wl_shm::Format;
use smithay::utils::Size;
use smithay::wayland::image_capture_source::{
    ImageCaptureSource, ImageCaptureSourceHandler, OutputCaptureSourceHandler,
    OutputCaptureSourceState,
};
use smithay::wayland::image_copy_capture::{
    BufferConstraints, Frame, ImageCopyCaptureHandler, ImageCopyCaptureState, Session, SessionRef,
};

impl ImageCaptureSourceHandler for State {
    fn source_destroyed(&mut self, source: ImageCaptureSource) {
        if let Some(id) = source.user_data().get::<CaptureSourceId>() {
            image_capture::on_source_destroyed(&mut self.wl.image_capture.book, *id);
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
        let id = source.user_data().get::<CaptureSourceId>()?;
        let output = self.wl.image_capture.book.sources.get(id)?.clone();
        constraints_of(self, &output)
    }

    fn new_session(&mut self, session: Session) {
        let source = session.source();
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
        frames::serve(self, session, frame);
    }

    fn session_destroyed(&mut self, session: SessionRef) {
        if let Some(id) = session.user_data().get::<CaptureSessionId>() {
            self.wl.image_capture.release(*id);
        }
    }
}

/// What a client must allocate to capture `output` now: its mode's size, in the two shared
/// memory formats every client has. (smithay's `dma` field is there because the backend
/// crate always builds smithay with `renderer_multi`, which implies `backend_drm`; dma-buf
/// capture follows, FINDINGS p1-c1.)
pub(super) fn constraints_of(state: &State, output: &OutputKey) -> Option<BufferConstraints> {
    let mode = state.core.model.outputs().0.get(output)?.config.mode.size;
    let side = |n: u32| i32::try_from(n).ok();
    Some(BufferConstraints {
        size: Size::from((side(mode.w)?, side(mode.h)?)),
        shm: vec![Format::Argb8888, Format::Xrgb8888],
        dma: None,
    })
}
