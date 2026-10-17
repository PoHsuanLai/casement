//! The consent gate of ext image copy capture frames (`Power::Screen`): the frame request is
//! where the screen is first read, so it is held until the person answers, served when they
//! allow and refused (the frame `failed`, the session `stopped`) when they do not.

use crate::state::State;
use casement_core::ClientKey;
use casement_core::privilege::PowerOutcome;
use casement_protocols::vocab::Power;
use casement_wayland::consent::{self, Admission};
use casement_wayland::image_capture::GatedFrame;
use smithay::wayland::image_copy_capture::{
    CaptureFailureReason, Frame, ImageCopyCaptureHandler, SessionRef,
};

/// Ends every capture session of `client`, output and window alike.
fn stop_sessions(state: &mut State, client: ClientKey) {
    state.wl.image_capture.stop_client(client);
    state.wl.capture_source.stop_client(client);
    state.wl.image_capture.cursors.stop_client(client);
}

/// `frame` when its client may capture now; otherwise it is kept until the answer comes (and
/// `None` is returned) or refused.
pub(super) fn pass(state: &mut State, session: &SessionRef, frame: Frame) -> Option<Frame> {
    let Some(client) = session.client() else {
        frame.fail(CaptureFailureReason::Stopped);
        return None;
    };
    let key = consent::key_of(&client);
    match (consent::admit(state, &client, Power::Screen), key) {
        (Admission::Go, _) => Some(frame),
        (Admission::Hold, Some(key)) => {
            state
                .wl
                .image_capture
                .gate_frame(key, session.clone(), frame);
            None
        }
        (Admission::Hold | Admission::Refuse, key) => {
            frame.fail(CaptureFailureReason::Stopped);
            if let Some(key) = key {
                stop_sessions(state, key);
            }
            None
        }
    }
}

/// The answer about `client`'s screen came out: the frame requests that waited are served, or
/// refused with their sessions; a power taken back stops the sessions in use.
pub(in crate::handlers) fn resolved(state: &mut State, client: ClientKey, outcome: PowerOutcome) {
    // The cursor sessions that waited (or, when refused, every one of the client's) go first.
    super::cursor::resolved(state, client, outcome);
    let gated = state.wl.image_capture.take_gated(client);
    match outcome {
        PowerOutcome::Allowed => {
            for held in gated {
                let (session, frame) = held.into_parts();
                ImageCopyCaptureHandler::frame(state, &session, frame);
            }
        }
        PowerOutcome::Denied | PowerOutcome::Revoked => {
            gated.into_iter().for_each(GatedFrame::refuse);
            stop_sessions(state, client);
        }
    }
}
