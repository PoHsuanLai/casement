//! Which targets an event of each origin may reach, for which use (I3, I8, the lock, the feed).

use super::Origin;
use crate::ids::{LayerKey, WindowId};
use crate::lock::LockState;

/// What an event would land on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputTarget {
    /// A window's surface tree.
    Window(WindowId),
    /// A popup of this window.
    Popup(WindowId),
    /// A layer surface.
    Layer(LayerKey),
    /// The session lock's surface.
    LockSurface,
    /// A trusted surface (the confirm sheet).
    Trusted,
    /// Nothing (the desktop background, with no wallpaper client).
    Nothing,
}

/// What the event would be used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputUse {
    /// A key press or release.
    Key,
    /// A button or touch press.
    Press,
    /// Pointer motion.
    Motion,
    /// A scroll.
    Scroll,
    /// A system gesture (a Space swipe, Mission Control).
    SystemGesture,
    /// Text typed through an input method or the agent.
    Text,
}

/// The state [`reach`] reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReachFacts<'a> {
    /// The session lock.
    pub lock: &'a LockState,
}

/// Whether an event may reach its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// It may.
    Allowed,
    /// It may not, for this reason.
    Refused(Refusal),
}

/// Why an event was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Refusal {
    /// Only physical input reaches this target (a trusted surface).
    NotPhysical,
    /// A feed may only scroll and gesture.
    FeedKind,
    /// The agent's target is outside its lease.
    OutsideLease,
    /// The agent's lease is suspended.
    Suspended,
    /// The kill chord ended the agent's leases.
    Killed,
    /// The session is locked and the event is not physical, or not for the lock surface.
    Locked,
}

/// Whether an event from `origin` may reach `target` for `use_`.
///
/// Rules: a feed scrolls and gestures only; while locked (or locking) only physical (and test)
/// input reaches only the lock surface; only physical (and test) input reaches a trusted
/// surface; virtual devices are never physical (I8); the agent reaches nothing until leases
/// arrive (P4-Z extends [`ReachFacts`]).
pub fn reach(origin: &Origin, target: &InputTarget, use_: InputUse, facts: &ReachFacts) -> Reach {
    let _ = (origin, target, use_, facts);
    todo!("P0-F1-input: the reach table")
}
