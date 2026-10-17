//! What the timing lines remember between the moments.

use super::first_show::{Awaiting, FirstShow, Opened};
use super::key_window::KeyChange;
use super::session_ready::SessionStage;
use super::sink::Sink;
use super::stamp::Stamp;
use super::trigger::{FeedKind, Press};
use super::xwayland_ready::XwaylandStage;
use casement_core::time::Mono;
use casement_core::{ClientKey, WindowId};
use std::collections::BTreeMap;

/// The timing lines' state, a field of `State`. Everything here is a lookup in a small ordered
/// map or a comparison: a frame with nothing to report asks [`Timing::is_idle`] and stops.
#[derive(Debug)]
pub struct Timing {
    pub(super) epoch: Stamp,
    pub(super) sink: Sink,
    pub(super) connected: BTreeMap<ClientKey, Stamp>,
    /// Toplevels that mapped and have not yet been reported.
    pub(super) shows: BTreeMap<WindowId, (ClientKey, FirstShow)>,
    /// Toplevels already reported, by the client that owns them, so a window that unmaps and maps
    /// again does not report a second first show. Dropped with the client.
    pub(super) reported: BTreeMap<WindowId, ClientKey>,
    /// The window the last reported key change made key.
    pub(super) key: Option<WindowId>,
    /// Key changes waiting for the next frame.
    pub(super) pending: Vec<KeyChange>,
    pub(super) press: Option<Press>,
    pub(super) feed: FeedKind,
    pub(super) xwayland: XwaylandStage,
    pub(super) session: SessionStage,
}

impl Timing {
    /// Timing lines for a clock whose zero read `epoch` on `CLOCK_MONOTONIC`
    /// (`ClockSource::monotonic_epoch_ns`). The session starts at that epoch.
    pub fn new(epoch: Stamp) -> Timing {
        Timing {
            epoch,
            sink: Sink::Journal,
            connected: BTreeMap::new(),
            shows: BTreeMap::new(),
            reported: BTreeMap::new(),
            key: None,
            pending: Vec::new(),
            press: None,
            feed: FeedKind::Plain,
            xwayland: XwaylandStage::Idle,
            session: SessionStage::starting(epoch),
        }
    }

    /// The same, keeping its lines for a test to read instead of writing them to the log.
    #[must_use]
    pub fn kept(self) -> Timing {
        Timing {
            sink: Sink::Kept(Vec::new()),
            ..self
        }
    }

    /// The lines kept so far (a test's timing); none otherwise.
    pub fn lines(&self) -> &[String] {
        self.sink.lines()
    }

    /// The moment `at` on the session clock, on `CLOCK_MONOTONIC`.
    pub fn stamp(&self, at: Mono) -> Stamp {
        Stamp::of(self.epoch, at)
    }

    /// Whether no frame has anything to tell: no window waits for its first show, no key change
    /// waits for a frame and the shell has no surface to show yet (none connected, or its start
    /// is over).
    pub fn is_idle(&self) -> bool {
        self.shows.is_empty() && self.pending.is_empty() && !self.session.watches_surfaces()
    }

    /// Whether frames should be looked at for the shell's bar and dock.
    pub fn watches_shell(&self) -> bool {
        self.session.watches_surfaces()
    }

    /// A client connected at `at`.
    pub fn client_connected(&mut self, client: ClientKey, at: Stamp) {
        self.connected.insert(client, at);
    }

    /// When `client` connected, if it is still connected.
    pub fn connected_at(&self, client: ClientKey) -> Option<Stamp> {
        self.connected.get(&client).copied()
    }

    /// A client went away: what is kept for it goes too.
    pub fn client_gone(&mut self, client: ClientKey) {
        self.connected.remove(&client);
        self.shows.retain(|_, (owner, _)| *owner != client);
        self.reported.retain(|_, owner| *owner != client);
    }

    /// A toplevel of `client` mapped. Only the first map of a window begins a first show.
    pub fn window_mapped(&mut self, window: WindowId, client: ClientKey, opened: Opened) {
        if self.reported.contains_key(&window) || self.shows.contains_key(&window) {
            return;
        }
        self.shows
            .insert(window, (client, FirstShow::committed(opened)));
    }

    /// A toplevel unmapped. One that was never shown starts again at its next map.
    pub fn window_unmapped(&mut self, window: WindowId) {
        self.shows.remove(&window);
        if self.key == Some(window) {
            self.key = None;
        }
    }

    /// The windows whose first show waits for `what`, in id order.
    pub fn awaiting(&self, what: Awaiting) -> Vec<WindowId> {
        self.shows
            .iter()
            .filter(|(_, (_, show))| show.awaiting() == what)
            .map(|(window, _)| *window)
            .collect()
    }

    /// A frame at `at` showed `window`.
    pub fn window_shown(&mut self, window: WindowId, at: Stamp) {
        if let Some((owner, show)) = self.shows.remove(&window) {
            self.shows.insert(window, (owner, show.shown(at)));
        }
    }

    /// A frame at `at` shows `window` at rest: its first show is over, and its line is written.
    pub fn window_rested(&mut self, window: WindowId, at: Stamp) {
        let Some((owner, show)) = self.shows.remove(&window) else {
            return;
        };
        match show.settled(at) {
            Ok(ready) => {
                self.reported.insert(window, owner);
                self.sink.write(ready.line());
            }
            Err(show) => {
                self.shows.insert(window, (owner, show));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timing::token::Token;
    use casement_core::Pid;

    fn opened() -> Opened {
        Opened {
            app_id: Token::of("foot"),
            pid: Pid(1),
            connected_at: Stamp(10),
            first_commit_at: Stamp(20),
        }
    }

    #[test]
    fn a_window_reports_its_first_show_once_even_if_it_maps_again() {
        let mut timing = Timing::new(Stamp::NONE).kept();
        let (client, window) = (ClientKey(1), WindowId(7));
        timing.window_mapped(window, client, opened());
        assert_eq!(timing.awaiting(Awaiting::Frame), vec![window]);
        // Rest before a frame shows it does nothing.
        timing.window_rested(window, Stamp(30));
        assert!(timing.lines().is_empty());
        timing.window_shown(window, Stamp(40));
        assert_eq!(timing.awaiting(Awaiting::Rest), vec![window]);
        timing.window_rested(window, Stamp(90));
        assert_eq!(
            timing.lines(),
            [
                "window-ready app_id=foot pid=1 connected_at=10 first_commit_at=20 first_frame_at=40 settled_at=90"
            ]
        );
        assert!(timing.is_idle());
        // It unmaps and maps again: no second line. Its client leaving forgets it.
        timing.window_unmapped(window);
        timing.window_mapped(window, client, opened());
        assert!(timing.awaiting(Awaiting::Frame).is_empty());
        timing.client_gone(client);
        timing.window_mapped(window, ClientKey(2), opened());
        assert_eq!(timing.awaiting(Awaiting::Frame), vec![window]);
    }

    #[test]
    fn a_key_change_waits_for_a_frame_and_is_written_once() {
        let mut timing = Timing::new(Stamp::NONE).kept();
        let window = WindowId(3);
        timing.key_changed(window, Token::of("foot"), Stamp(100));
        // Made key again, and a frame is waited for: still one change.
        timing.key_changed(window, Token::of("foot"), Stamp(110));
        assert!(!timing.is_idle());
        timing.keys_presented(Stamp(150));
        timing.keys_presented(Stamp(180));
        assert_eq!(
            timing.lines(),
            ["key-window app_id=foot cause=other input_at=0 focused_at=100 presented_at=150"]
        );
        // Losing the key and getting it back is a second change.
        timing.key_lost(window);
        timing.key_changed(window, Token::of("foot"), Stamp(200));
        timing.keys_presented(Stamp(210));
        assert_eq!(timing.lines().len(), 2);
    }
}
