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

/// Who asked for the lock that is held now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockHolder {
    /// No lock is held.
    Nobody,
    /// The shell's own locker. `None` while the lock is one casement restored after a restart,
    /// before the shell's new connection has locked again. Only the shell's locks are
    /// remembered across a restart.
    Shell(Option<ClientKey>),
    /// Any other client.
    Client(ClientKey),
}

impl LockHolder {
    /// The client that asked for the lock, if it is known.
    pub fn client(self) -> Option<ClientKey> {
        match self {
            LockHolder::Nobody | LockHolder::Shell(None) => None,
            LockHolder::Shell(Some(client)) | LockHolder::Client(client) => Some(client),
        }
    }
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
    /// The locker's surface on an output is gone (destroyed, or committed without a buffer):
    /// the output shows the cover again.
    SurfaceGone {
        /// The output.
        output: OutputKey,
    },
    /// The locker unlocked.
    Unlocked,
    /// The locker's connection went away while locked.
    LockerGone,
    /// casement started and found the lock marker its last run left: the session was locked by
    /// the shell, and stays covered until the shell locks again and unlocks.
    Restored,
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
    /// Write the lock marker: a restart from here comes back locked.
    Remember,
    /// Remove the lock marker: the session is not locked by the shell any more.
    Forget,
}
