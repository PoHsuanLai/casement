//! A toplevel's first show, filled in order: connected, first commit, first frame, settled.

use super::stamp::Stamp;
use super::token::Token;
use super::window_ready::WindowReady;
use casement_core::Pid;

/// What is known when a toplevel maps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    /// The app id its client gave.
    pub app_id: Token,
    /// Its client's process, or 0.
    pub pid: Pid,
    /// When its client connected.
    pub connected_at: Stamp,
    /// Its first commit with a buffer.
    pub first_commit_at: Stamp,
}

/// What a first show waits for next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Awaiting {
    /// The first frame that shows the window.
    Frame,
    /// The end of its open animation.
    Rest,
}

/// One toplevel's first show: it fills its stamps in order and reports once (`settled` gives the
/// finished record and the show is over).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstShow {
    /// Mapped, not yet on a frame.
    Committed(Opened),
    /// On a frame, its open animation not yet at rest.
    Shown {
        /// What was known at the map.
        opened: Opened,
        /// The first frame that showed it.
        first_frame_at: Stamp,
    },
}

impl FirstShow {
    /// A toplevel that just mapped. The stamps are put in order: a commit is not before its
    /// connection.
    pub fn committed(opened: Opened) -> FirstShow {
        let first_commit_at = opened.first_commit_at.or_later(opened.connected_at);
        FirstShow::Committed(Opened {
            first_commit_at,
            ..opened
        })
    }

    /// What it waits for.
    pub fn awaiting(&self) -> Awaiting {
        match self {
            FirstShow::Committed(_) => Awaiting::Frame,
            FirstShow::Shown { .. } => Awaiting::Rest,
        }
    }

    /// A frame at `at` showed the window. Only the first counts; a show already on a frame keeps
    /// its stamp.
    #[must_use]
    pub fn shown(self, at: Stamp) -> FirstShow {
        match self {
            FirstShow::Committed(opened) => {
                let first_frame_at = at.or_later(opened.first_commit_at);
                FirstShow::Shown {
                    opened,
                    first_frame_at,
                }
            }
            shown @ FirstShow::Shown { .. } => shown,
        }
    }

    /// The open animation is at rest in a frame at `at`: the finished record. A window not yet
    /// on a frame is not at rest yet and comes back unchanged. With no animation the first frame
    /// is also the one at rest, so `settled_at` equals `first_frame_at`.
    pub fn settled(self, at: Stamp) -> Result<WindowReady, FirstShow> {
        match self {
            FirstShow::Shown {
                opened,
                first_frame_at,
            } => Ok(WindowReady {
                app_id: opened.app_id,
                pid: opened.pid,
                connected_at: opened.connected_at,
                first_commit_at: opened.first_commit_at,
                first_frame_at,
                settled_at: at.or_later(first_frame_at),
            }),
            committed @ FirstShow::Committed(_) => Err(committed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened() -> Opened {
        Opened {
            app_id: Token::of("foot"),
            pid: Pid(99),
            connected_at: Stamp(100),
            first_commit_at: Stamp(250),
        }
    }

    fn done(first_frame_at: u64, settled_at: u64) -> WindowReady {
        WindowReady {
            app_id: Token::of("foot"),
            pid: Pid(99),
            connected_at: Stamp(100),
            first_commit_at: Stamp(250),
            first_frame_at: Stamp(first_frame_at),
            settled_at: Stamp(settled_at),
        }
    }

    #[test]
    fn the_stamps_fill_in_order_and_the_record_is_given_once() {
        let animated = FirstShow::committed(opened())
            .shown(Stamp(900))
            // A later frame does not move the first one.
            .shown(Stamp(950));
        assert_eq!(animated.awaiting(), Awaiting::Rest);
        assert_eq!(animated.settled(Stamp(1_400)), Ok(done(900, 1_400)));
    }

    #[test]
    fn without_an_animation_the_first_frame_is_the_settled_one() {
        let still = FirstShow::committed(opened()).shown(Stamp(900));
        assert_eq!(still.settled(Stamp(900)), Ok(done(900, 900)));
    }

    #[test]
    fn a_window_not_yet_on_a_frame_is_not_settled() {
        let waiting = FirstShow::committed(opened());
        assert_eq!(waiting.awaiting(), Awaiting::Frame);
        assert_eq!(waiting.clone().settled(Stamp(5_000)), Err(waiting));
    }

    #[test]
    fn out_of_order_clock_reads_are_clamped_forward() {
        // A commit stamped before the connection, a frame before the commit, a rest before the frame.
        let early = FirstShow::committed(Opened {
            first_commit_at: Stamp(50),
            ..opened()
        })
        .shown(Stamp(120));
        let want = WindowReady {
            first_commit_at: Stamp(100),
            ..done(120, 120)
        };
        assert_eq!(early.settled(Stamp(10)), Ok(want));
    }
}
