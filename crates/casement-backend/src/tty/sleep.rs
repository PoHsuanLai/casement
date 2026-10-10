//! Suspend reaching the session (P1-T): logind's `PrepareForSleep` is heard on the system bus
//! by a thread at the binary's edge, which pushes `Paused` and `Activated` into the session's
//! queue through a [`SleepLink`]. libseat does not report a suspend, so without this the
//! displays would come back from sleep still believed lit and nothing would re-modeset them.
//!
//! The thread holds a delay inhibitor while it waits: [`SleepLink::paused`] tells the session
//! the machine is about to sleep and then waits (for a bounded time) until the backend has acted
//! on the pause ([`SleepInbox::settled`]), so devices are released before logind lets the sleep
//! go on.
//!
//! ```
//! use casement_backend::tty::{SessionNote, SleepSettled, sleep_channel};
//! use std::time::Duration;
//!
//! let (link, inbox) = sleep_channel(|| {});
//! let loop_side = std::thread::spawn(move || {
//!     let heard = inbox.take_blocking(Duration::from_secs(5));
//!     assert_eq!(heard, [SessionNote::Paused]);
//!     inbox.settled();
//!     inbox
//! });
//! assert_eq!(link.paused(Duration::from_secs(5)), SleepSettled::Done);
//! let inbox = loop_side.join().expect("the loop side");
//! link.activated();
//! assert_eq!(inbox.take(), [SessionNote::Activated]);
//! ```

use super::session::SessionNote;
use std::fmt;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::Duration;

/// What waiting for the backend to act on a pause came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SleepSettled {
    /// The backend acted on the pause.
    Done,
    /// It did not within the time allowed (the sleep goes on anyway).
    TimedOut,
    /// The compositor is gone.
    Gone,
}

/// The edge's end: tell the session about a suspend.
pub struct SleepLink {
    notes: Sender<SessionNote>,
    wake: Arc<dyn Fn() + Send + Sync>,
    settled: Receiver<()>,
}

/// The session's end: the notes the edge sent, and the way to say they were acted on.
pub struct SleepInbox {
    notes: Receiver<SessionNote>,
    settled: Sender<()>,
}

/// A link and its inbox. `wake` is called after every note is sent, to make the event loop turn
/// (the real session pings its loop; a test passes a no-op).
pub fn sleep_channel(wake: impl Fn() + Send + Sync + 'static) -> (SleepLink, SleepInbox) {
    let (notes_tx, notes_rx) = channel();
    let (settled_tx, settled_rx) = channel();
    (
        SleepLink {
            notes: notes_tx,
            wake: Arc::new(wake),
            settled: settled_rx,
        },
        SleepInbox {
            notes: notes_rx,
            settled: settled_tx,
        },
    )
}

impl SleepLink {
    /// The machine is about to sleep: sends `Paused`, then waits at most `wait` for the backend
    /// to act on it.
    pub fn paused(&self, wait: Duration) -> SleepSettled {
        // An acknowledgement left over from an earlier pause must not count for this one.
        while self.settled.try_recv().is_ok() {}
        if self.notes.send(SessionNote::Paused).is_err() {
            return SleepSettled::Gone;
        }
        (self.wake)();
        match self.settled.recv_timeout(wait) {
            Ok(()) => SleepSettled::Done,
            Err(RecvTimeoutError::Timeout) => SleepSettled::TimedOut,
            Err(RecvTimeoutError::Disconnected) => SleepSettled::Gone,
        }
    }

    /// The machine woke: sends `Activated`. Whether the compositor is still there is not the
    /// edge's to act on; a send to a compositor that left is dropped.
    pub fn activated(&self) {
        if self.notes.send(SessionNote::Activated).is_ok() {
            (self.wake)();
        }
    }
}

impl SleepInbox {
    /// The notes sent since the last call, oldest first.
    pub fn take(&self) -> Vec<SessionNote> {
        self.notes.try_iter().collect()
    }

    /// The next note, waiting at most `wait` (a test thread standing in for the loop).
    pub fn take_blocking(&self, wait: Duration) -> Vec<SessionNote> {
        self.notes
            .recv_timeout(wait)
            .into_iter()
            .chain(self.notes.try_iter())
            .collect()
    }

    /// The backend acted on a `Paused` note: its devices are released.
    pub fn settled(&self) {
        // The edge may have given up waiting and gone; that is not an error here.
        let _ = self.settled.send(());
    }
}

impl fmt::Debug for SleepLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SleepLink")
    }
}

impl fmt::Debug for SleepInbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SleepInbox")
    }
}
