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
/// The checks run in this order, the first that refuses wins:
///
/// 1. **The lock.** While the session is locked (or locking), only physical (and test) input
///    reaches only the lock surface; everything else is [`Refusal::Locked`], whoever sends it.
/// 2. **The agent.** The agent reaches nothing until leases arrive (P4-Z extends
///    [`ReachFacts`]): [`Refusal::OutsideLease`].
/// 3. **The feed.** A feed scrolls and gestures only: [`Refusal::FeedKind`].
/// 4. **Trusted surfaces.** Only physical (and test) input reaches one (I3), and virtual
///    devices are never physical (I8): [`Refusal::NotPhysical`].
///
/// Everything else is allowed: a remote session or a virtual keyboard types into windows like
/// the person would.
pub fn reach(origin: &Origin, target: &InputTarget, use_: InputUse, facts: &ReachFacts) -> Reach {
    [
        refuse_by_lock(origin, target, *facts),
        refuse_agent(origin),
        refuse_feed(origin, use_),
        refuse_untrusted(origin, target),
    ]
    .into_iter()
    .flatten()
    .next()
    .map_or(Reach::Allowed, Reach::Refused)
}

fn refuse_by_lock(origin: &Origin, target: &InputTarget, facts: ReachFacts) -> Option<Refusal> {
    match facts.lock {
        LockState::Unlocked => None,
        LockState::Locking { .. }
        | LockState::Locked { .. }
        | LockState::LockedWithoutSurface { .. } => {
            let reaches_lock = origin.is_physical() && matches!(target, InputTarget::LockSurface);
            (!reaches_lock).then_some(Refusal::Locked)
        }
    }
}

fn refuse_agent(origin: &Origin) -> Option<Refusal> {
    matches!(origin, Origin::Agent(_)).then_some(Refusal::OutsideLease)
}

fn refuse_feed(origin: &Origin, use_: InputUse) -> Option<Refusal> {
    let allowed = matches!(use_, InputUse::Scroll | InputUse::SystemGesture);
    (matches!(origin, Origin::Feed(_)) && !allowed).then_some(Refusal::FeedKind)
}

fn refuse_untrusted(origin: &Origin, target: &InputTarget) -> Option<Refusal> {
    (matches!(target, InputTarget::Trusted) && !origin.is_physical())
        .then_some(Refusal::NotPhysical)
}
