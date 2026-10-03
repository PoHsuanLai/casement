//! The lock's states. Once locked, the session stays covered until the locker unlocks: a
//! locker that dies leaves the cover up (`LockedWithoutSurface`), never the desktop.

use crate::ids::{ClientKey, OutputKey};
use crate::time::Mono;

/// Where the session lock stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockState {
    /// Not locked.
    Unlocked,
    /// A locker asked; outputs are going dark.
    Locking {
        /// When it asked.
        since: Mono,
    },
    /// Locked, with the locker's surfaces.
    Locked {
        /// Which outputs its surfaces cover.
        surfaces: LockSurfaces,
    },
    /// Locked, and the locker is gone: every output shows the lock cover.
    LockedWithoutSurface,
}

/// Which outputs the locker has covered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockSurfaces {
    /// None committed yet.
    Pending,
    /// These.
    Covering(Vec<OutputKey>),
}

/// What moves the lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockEvent {
    /// A client asked to lock.
    Requested {
        /// The locker.
        client: ClientKey,
    },
    /// The locker committed its surface on an output.
    SurfaceCommitted {
        /// The output.
        output: OutputKey,
    },
    /// The locker unlocked.
    Unlocked,
    /// The locker's connection went away while locked.
    LockerGone,
}

/// What the lock asks the edge to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockReply {
    /// Send `locked` to the locker.
    SendLocked,
    /// Draw the lock cover on this output.
    ShowCover(OutputKey),
    /// Stop drawing the cover on this output.
    HideCover(OutputKey),
    /// The lock is over (send `finished` if the locker remains).
    Finished,
}
