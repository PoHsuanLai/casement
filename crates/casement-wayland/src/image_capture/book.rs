//! What the capture managers know, without a wire: the sources made, the sessions running and
//! the frames asked for. Pure, so the bookkeeping is tested without a display.

use super::{CaptureFailure, CaptureRequest, CaptureSessionId, CaptureSourceId};
use crate::states::WaylandFault;
use casement_core::OutputKey;
use casement_core::geometry::PhysicalSize;
use casement_core::time::Mono;
use std::collections::BTreeMap;

/// The sources, sessions and pending frames.
#[derive(Debug, Default)]
pub struct CaptureBook {
    /// Each source and the output it shows.
    pub sources: BTreeMap<CaptureSourceId, OutputKey>,
    /// Each session and the output it captures (a session outlives its source: the client may
    /// destroy the source right after creating the session).
    pub sessions: BTreeMap<CaptureSessionId, OutputKey>,
    /// Each session with a frame asked for and not answered, and the buffer size it came with.
    pub pending: BTreeMap<CaptureSessionId, PhysicalSize>,
    /// The instant of the last frame each session was given.
    pub delivered: BTreeMap<CaptureSessionId, Mono>,
    /// The next id to hand out.
    pub next: u64,
}

impl CaptureBook {
    /// The next id's number.
    fn mint(&mut self) -> u64 {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }

    /// Notes a source for `output`.
    pub fn add_source(&mut self, output: OutputKey) -> CaptureSourceId {
        let id = CaptureSourceId(self.mint());
        self.sources.insert(id, output);
        id
    }

    /// Forgets a source the client destroyed (its sessions go on).
    pub fn drop_source(&mut self, source: CaptureSourceId) {
        self.sources.remove(&source);
    }

    /// Notes a session on `source`.
    pub fn add_session(
        &mut self,
        source: CaptureSourceId,
    ) -> Result<CaptureSessionId, WaylandFault> {
        let output = self
            .sources
            .get(&source)
            .cloned()
            .ok_or(WaylandFault::UnknownCapture(source.0))?;
        let id = CaptureSessionId(self.mint());
        self.sessions.insert(id, output);
        Ok(id)
    }

    /// Forgets a session and everything about it.
    pub fn drop_session(&mut self, session: CaptureSessionId) {
        self.sessions.remove(&session);
        self.pending.remove(&session);
        self.delivered.remove(&session);
    }

    /// A frame asked for on `session` with a buffer of `buffer`.
    pub fn request_frame(
        &mut self,
        session: CaptureSessionId,
        buffer: PhysicalSize,
    ) -> Result<CaptureRequest, WaylandFault> {
        let output = self
            .sessions
            .get(&session)
            .cloned()
            .ok_or(WaylandFault::UnknownCapture(session.0))?;
        self.pending.insert(session, buffer);
        Ok(CaptureRequest {
            output,
            size: buffer,
        })
    }

    /// The pending frame of `session` was delivered at `at`.
    pub fn frame_ready(&mut self, session: CaptureSessionId, at: Mono) -> Result<(), WaylandFault> {
        self.pending
            .remove(&session)
            .ok_or(WaylandFault::UnknownCapture(session.0))?;
        self.delivered.insert(session, at);
        Ok(())
    }

    /// The pending frame of `session` failed. A session whose output is gone is over.
    pub fn frame_failed(
        &mut self,
        session: CaptureSessionId,
        why: CaptureFailure,
    ) -> Result<(), WaylandFault> {
        self.pending
            .remove(&session)
            .ok_or(WaylandFault::UnknownCapture(session.0))?;
        if why == CaptureFailure::OutputGone {
            self.drop_session(session);
        }
        Ok(())
    }

    /// Forgets every source and session on `output`; the sessions it had.
    pub fn forget_output(&mut self, output: &OutputKey) -> Vec<CaptureSessionId> {
        self.sources.retain(|_, shown| *shown != *output);
        let gone: Vec<CaptureSessionId> = self
            .sessions
            .iter()
            .filter(|(_, captured)| *captured == output)
            .map(|(id, _)| *id)
            .collect();
        for id in &gone {
            self.drop_session(*id);
        }
        gone
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::OutputName;
    use casement_core::output::{EdidFacts, EdidSerial};
    use std::time::Duration;

    fn key(name: &str) -> OutputKey {
        let edid = EdidFacts {
            make: "Test".to_owned(),
            model: "Panel".to_owned(),
            serial: EdidSerial::Unknown,
        };
        OutputKey::of(&edid, &OutputName::parse(name).expect("a name"))
    }

    const SIZE: PhysicalSize = PhysicalSize { w: 640, h: 480 };

    #[test]
    fn a_frame_goes_from_request_to_ready() {
        let mut book = CaptureBook::default();
        let source = book.add_source(key("HEADLESS-1"));
        let session = book.add_session(source).expect("session");
        let request = book.request_frame(session, SIZE).expect("request");
        assert_eq!(request.output, key("HEADLESS-1"));
        assert_eq!(request.size, SIZE);
        let at = Mono(Duration::from_millis(16));
        book.frame_ready(session, at).expect("ready");
        assert_eq!(book.delivered.get(&session), Some(&at));
        assert!(book.frame_ready(session, at).is_err(), "answered once");
    }

    #[test]
    fn a_session_outlives_its_source() {
        let mut book = CaptureBook::default();
        let source = book.add_source(key("HEADLESS-1"));
        let session = book.add_session(source).expect("session");
        book.drop_source(source);
        assert!(book.request_frame(session, SIZE).is_ok());
        assert!(
            book.add_session(source).is_err(),
            "no session on a gone source"
        );
    }

    #[test]
    fn an_unplugged_output_takes_its_sessions_and_only_them() {
        let mut book = CaptureBook::default();
        let (one, two) = (book.add_source(key("A-1")), book.add_source(key("B-1")));
        let on_one = book.add_session(one).expect("session");
        let on_two = book.add_session(two).expect("session");
        book.request_frame(on_one, SIZE).expect("request");
        assert_eq!(book.forget_output(&key("A-1")), vec![on_one]);
        assert!(!book.pending.contains_key(&on_one));
        assert!(book.sessions.contains_key(&on_two));
        assert!(book.sources.contains_key(&two) && !book.sources.contains_key(&one));
    }

    #[test]
    fn a_failure_for_a_gone_output_ends_the_session() {
        let mut book = CaptureBook::default();
        let source = book.add_source(key("A-1"));
        let kept = book.add_session(source).expect("session");
        let ended = book.add_session(source).expect("session");
        for session in [kept, ended] {
            book.request_frame(session, SIZE).expect("request");
        }
        book.frame_failed(kept, CaptureFailure::BufferMismatch)
            .expect("failed");
        book.frame_failed(ended, CaptureFailure::OutputGone)
            .expect("failed");
        assert!(book.sessions.contains_key(&kept));
        assert!(!book.sessions.contains_key(&ended));
    }
}
