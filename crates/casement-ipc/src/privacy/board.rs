//! The Privacy list as the bus thread reads it, and the way Settings' changes reach the event
//! loop. The model owns the answers; the loop publishes [`Privilege::rows`] to the board after
//! every step that changes them, and the bus thread serves what the board holds.
//!
//! [`Privilege::rows`]: casement_core::privilege::Privilege::rows

use calloop::channel::{Channel, SyncSender, sync_channel};
use casement_core::privilege::{Edit, PrivacyRow};
use std::sync::Arc;
use tokio::sync::watch;

/// The changes waiting for the event loop. Settings makes one at a time, by hand; a change
/// beyond these is refused with "try again", never queued without bound.
pub const EDIT_QUEUE: usize = 16;

/// The rows of the Privacy list, shared between the event loop (which publishes) and the bus
/// thread (which serves and follows).
#[derive(Debug, Clone)]
pub struct PrivacyBoard {
    rows: Arc<watch::Sender<Arc<Vec<PrivacyRow>>>>,
}

impl Default for PrivacyBoard {
    fn default() -> PrivacyBoard {
        PrivacyBoard::new()
    }
}

impl PrivacyBoard {
    /// A board with no rows.
    pub fn new() -> PrivacyBoard {
        let (rows, _) = watch::channel(Arc::new(Vec::new()));
        PrivacyBoard {
            rows: Arc::new(rows),
        }
    }

    /// Replaces the list; the bus thread tells Settings what differs. Does nothing when the
    /// list is the one the board has.
    pub fn publish(&self, rows: Vec<PrivacyRow>) {
        self.rows.send_if_modified(|current| {
            if **current == rows {
                return false;
            }
            *current = Arc::new(rows);
            true
        });
    }

    /// The list now.
    pub fn rows(&self) -> Arc<Vec<PrivacyRow>> {
        Arc::clone(&self.rows.borrow())
    }

    /// A receiver that wakes when the list changes.
    pub fn subscribe(&self) -> watch::Receiver<Arc<Vec<PrivacyRow>>> {
        self.rows.subscribe()
    }
}

/// The two halves the composition root and the bus thread share: the board, and the queue of
/// Settings' changes into the event loop.
#[derive(Debug, Clone)]
pub struct PrivacyLink {
    pub(crate) board: PrivacyBoard,
    pub(crate) edits: SyncSender<Edit>,
}

impl PrivacyLink {
    /// A new link, and the channel the event loop reads the changes from (the root turns each into
    /// an event, stamped with the wall-clock second).
    pub fn new() -> (PrivacyLink, Channel<Edit>) {
        let (edits, channel) = sync_channel(EDIT_QUEUE);
        let link = PrivacyLink {
            board: PrivacyBoard::new(),
            edits,
        };
        (link, channel)
    }

    /// The board, for the event loop to publish to.
    pub fn board(&self) -> PrivacyBoard {
        self.board.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::policy::Power;
    use casement_core::privilege::{ProgramPath, Source, Subject, UnixSeconds, Verdict};

    fn row() -> PrivacyRow {
        PrivacyRow {
            power: Power::Screen,
            subject: Subject::Program(ProgramPath::parse("/usr/bin/grim").expect("a path")),
            verdict: Verdict::Allow,
            source: Source::Person,
            at: Some(UnixSeconds(1)),
        }
    }

    #[test]
    fn publishing_the_same_list_wakes_nobody() {
        let board = PrivacyBoard::new();
        let mut listener = board.subscribe();
        board.publish(vec![row()]);
        assert!(listener.has_changed().expect("open"));
        listener.mark_unchanged();
        board.publish(vec![row()]);
        assert!(!listener.has_changed().expect("open"));
        assert_eq!(board.rows().len(), 1);
    }
}
