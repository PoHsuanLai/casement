//! The session lock (ext-session-lock): its states, what moves them, and what they ask for.

#[allow(
    clippy::module_inception,
    reason = "the freeze plan names the record file after its concept"
)]
mod lock;

pub use lock::{LockEvent, LockHolder, LockReply, LockState, LockSurfaces};
