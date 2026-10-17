//! The key window's changes, and what they are judged by.

use super::book::Timing;
use super::key_window::KeyChange;
use super::stamp::Stamp;
use super::token::Token;
use super::trigger::{FeedKind, Press, cause_of};
use casement_core::WindowId;

impl Timing {
    /// A turn of the loop begins with `feed` as its kind and, if it heard a press, that press.
    /// Returns the kind the enclosing turn had, for [`Timing::leave`] (turns nest).
    pub fn enter(&mut self, feed: FeedKind, press: Option<Press>) -> FeedKind {
        self.press = press.or(self.press);
        std::mem::replace(&mut self.feed, feed)
    }

    /// The turn that returned `outer` from [`Timing::enter`] ends.
    pub fn leave(&mut self, outer: FeedKind) {
        self.feed = outer;
    }

    /// `window` (of the app `app_id`) became the key window at `at`. Nothing is written when it
    /// already was; the line waits for the next frame ([`Timing::keys_presented`]).
    pub fn key_changed(&mut self, window: WindowId, app_id: Token, at: Stamp) {
        if self.key == Some(window) {
            return;
        }
        self.key = Some(window);
        let (cause, input_at) = cause_of(self.feed, self.press, at);
        self.pending.push(KeyChange {
            app_id,
            cause,
            input_at,
            focused_at: at,
        });
    }

    /// `window` stopped being key without another taking its place.
    pub fn key_lost(&mut self, window: WindowId) {
        if self.key == Some(window) {
            self.key = None;
        }
    }

    /// A frame presented at `at`: every key change waiting for one is written.
    pub fn keys_presented(&mut self, at: Stamp) {
        for change in std::mem::take(&mut self.pending) {
            self.sink.write(change.presented(at).line());
        }
    }
}
