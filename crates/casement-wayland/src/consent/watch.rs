//! The program behind a connection may run another image than the one it was vouched for at
//! accept. The composition root, which holds the pinned peer, says whether it does.

use casement_core::privilege::ClientStanding;
use std::fmt::Debug;

/// Looks at the image the process behind one connection runs now.
pub trait ImageWatch: Debug + Send + Sync {
    /// The standing the connection has now when it is no longer the one it had at accept (the
    /// program executed another image): [`ClientStanding::Unverified`], with the new image's
    /// file name. `None` while the image is the same one, and when it cannot be read: a
    /// process that is gone is the model's to refuse.
    fn moved(&self) -> Option<ClientStanding>;
}

/// The watch of a connection nobody looks at: its image never moves.
#[derive(Debug, Clone, Copy, Default)]
pub struct Unwatched;

impl ImageWatch for Unwatched {
    fn moved(&self) -> Option<ClientStanding> {
        None
    }
}
