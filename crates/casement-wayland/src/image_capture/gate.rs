//! The consent gate of `ext_image_copy_capture` (`Power::Screen`), for output sessions and
//! window sessions alike: a session may be made and told its constraints at once, a frame
//! request waits for the person's answer, and the denial is `stopped` for the session and
//! `failed` for the frame. A power taken back stops every session of the client, mid-capture
//! included.

use super::{CaptureSessionId, ImageCapturePart};
use crate::consent::{self, Held, Mortal, Pushed};
use casement_core::ClientKey;
use smithay::utils::IsAlive;
use smithay::wayland::image_copy_capture::{CaptureFailureReason, Frame, Session, SessionRef};

/// A frame request that waits for the person.
#[derive(Debug)]
pub struct GatedFrame {
    session: SessionRef,
    frame: Frame,
}

impl GatedFrame {
    /// The request is refused: the frame fails as the session stops.
    pub fn refuse(self) {
        self.frame.fail(CaptureFailureReason::Stopped);
    }

    /// The session and the frame, to be served.
    pub fn into_parts(self) -> (SessionRef, Frame) {
        (self.session, self.frame)
    }
}

impl Mortal for GatedFrame {
    fn alive(&self) -> bool {
        self.session.alive() && !self.frame.has_failed()
    }
}

/// The frame requests that wait for the person, by client.
pub type GatedFrames = Held<GatedFrame>;

/// The client a session belongs to, when casement inserted it.
pub fn owner_of(session: &Session) -> Option<ClientKey> {
    session.client().as_ref().and_then(consent::key_of)
}

impl ImageCapturePart {
    /// Keeps the frame request `frame` of `session` until the answer about `client`'s screen
    /// comes out; a client with too many waiting has the request refused.
    pub fn gate_frame(&mut self, client: ClientKey, session: SessionRef, frame: Frame) {
        if let Pushed::Full(gated) = self.gated.push(client, GatedFrame { session, frame }) {
            gated.refuse();
        }
    }

    /// The frame requests `client` has waiting, oldest first.
    pub fn take_gated(&mut self, client: ClientKey) -> Vec<GatedFrame> {
        self.gated.take(client)
    }

    /// Ends every output session of `client`: each is dropped, which tells the client
    /// `stopped` (and fails the frame it waits for).
    pub fn stop_client(&mut self, client: ClientKey) {
        let ended: Vec<CaptureSessionId> = self
            .held
            .iter()
            .filter(|(_, session)| owner_of(session) == Some(client))
            .map(|(id, _)| *id)
            .collect();
        for id in ended {
            // The session first: its drop fails the waiting frame with `stopped`.
            self.held.remove(&id);
            self.waiting.remove(&id);
            self.book.drop_session(id);
        }
        self.copy.cleanup();
    }
}
