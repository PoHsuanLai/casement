//! What tells one remote connection from another, and what ends one (CASW-19, PORTAL-05).
//!
//! Every `ConnectEis` gets a [`RemoteSlot`] minted by the compositor, one per connection and
//! never reused, and the virtual devices of that connection are numbered from the slot, so two
//! portal sessions never share a device id (the portal's session id is only the *origin*; it
//! used to be squeezed into the device id and lost its high bits). The keys and buttons a
//! connection holds down are shared between its event source and the [`RemoteHandle`] the
//! compositor keeps, so ending a session from outside can release them.

use crate::eis::RemoteInput;
use casement_core::DeviceId;
use casement_core::input::{
    ButtonCode, DeviceClass, Edge, InputKind, KeyCode, Origin, PortalSessionId,
};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};

/// How many remote connections one session of the compositor can number: a device id holds the
/// slot in 28 bits (the top bit keeps these ids clear of the real input backends'). Past it a
/// new connection is refused, never given a number already used.
pub const MAX_REMOTE_SLOTS: u32 = 1 << 28;

/// The most keys, and the most buttons, one connection may hold down at once: a keyboard has
/// fewer than 256 keys, so a peer that "holds" more is broken or hostile, and its presses are
/// dropped instead of growing the set without end.
pub const MAX_HELD: usize = 256;

/// The number of one remote connection among all the compositor has opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RemoteSlot(u32);

impl RemoteSlot {
    /// The slot's number.
    pub fn number(self) -> u32 {
        self.0
    }
}

/// Mints [`RemoteSlot`]s in order from 0; a slot is never reused in a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RemoteSlots {
    next: u32,
}

impl RemoteSlots {
    /// The next slot and the mint after it; none once [`MAX_REMOTE_SLOTS`] are used.
    pub fn next(self) -> Option<(RemoteSlots, RemoteSlot)> {
        (self.next < MAX_REMOTE_SLOTS).then_some((
            RemoteSlots {
                next: self.next.saturating_add(1),
            },
            RemoteSlot(self.next),
        ))
    }
}

/// Whether a press fitted in the held set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fit {
    /// Tracked (or a release).
    Held,
    /// The set is full ([`MAX_HELD`]); the press is to be dropped.
    Full,
}

#[derive(Debug, Default)]
struct HeldSets {
    keys: BTreeSet<u32>,
    buttons: BTreeSet<u32>,
}

/// The keys and buttons a connection holds down, shared by its source and its handle.
#[derive(Debug, Clone, Default)]
pub(crate) struct Held(Arc<Mutex<HeldSets>>);

impl PartialEq for Held {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Held {}

impl Held {
    fn sets(&self) -> std::sync::MutexGuard<'_, HeldSets> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Notes key `code` going to `edge`.
    pub(crate) fn key(&self, code: u32, edge: Edge) -> Fit {
        let mut sets = self.sets();
        note(&mut sets.keys, code, edge)
    }

    /// Notes button `code` going to `edge`.
    pub(crate) fn button(&self, code: u32, edge: Edge) -> Fit {
        let mut sets = self.sets();
        note(&mut sets.buttons, code, edge)
    }

    /// Forgets everything held and returns it: (keys, buttons).
    fn take(&self) -> (BTreeSet<u32>, BTreeSet<u32>) {
        let mut sets = self.sets();
        (
            std::mem::take(&mut sets.keys),
            std::mem::take(&mut sets.buttons),
        )
    }
}

fn note(set: &mut BTreeSet<u32>, code: u32, edge: Edge) -> Fit {
    match edge {
        Edge::Press if set.len() >= MAX_HELD && !set.contains(&code) => Fit::Full,
        Edge::Press => {
            set.insert(code);
            Fit::Held
        }
        Edge::Release => {
            set.remove(&code);
            Fit::Held
        }
    }
}

/// The id of the virtual device of `class` on connection `slot`. The high bit keeps these
/// clear of the ids the real input backends count up from.
pub fn device_id(slot: RemoteSlot, class: DeviceClass) -> DeviceId {
    let kind = match class {
        DeviceClass::Keyboard => 0,
        DeviceClass::Touch => 2,
        _ => 1,
    };
    DeviceId(0x8000_0000 | (slot.0 << 2) | kind)
}

/// An input event of portal session `session` on connection `slot`, tagged remote.
pub(crate) fn tagged(
    session: PortalSessionId,
    slot: RemoteSlot,
    class: DeviceClass,
    kind: InputKind,
) -> RemoteInput {
    RemoteInput {
        origin: Origin::Remote(session),
        device: device_id(slot, class),
        kind,
    }
}

/// The compositor's hold on one open remote connection: which session it is for, and enough to
/// release what the app still holds down when the session ends from outside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteHandle {
    session: PortalSessionId,
    slot: RemoteSlot,
    held: Held,
}

impl RemoteHandle {
    pub(crate) fn new(session: PortalSessionId, slot: RemoteSlot, held: Held) -> RemoteHandle {
        RemoteHandle {
            session,
            slot,
            held,
        }
    }

    /// The portal session the connection is for.
    pub fn session(&self) -> PortalSessionId {
        self.session
    }

    /// The connection's slot.
    pub fn slot(&self) -> RemoteSlot {
        self.slot
    }

    /// The releases of every key and button the app still holds, once: a second call (or the
    /// source ending afterwards) finds nothing held.
    pub fn release(&self) -> Vec<RemoteInput> {
        releases(self.session, self.slot, &self.held)
    }
}

/// The releases of what `held` holds, which is forgotten.
pub(crate) fn releases(
    session: PortalSessionId,
    slot: RemoteSlot,
    held: &Held,
) -> Vec<RemoteInput> {
    let (keys, buttons) = held.take();
    keys.into_iter()
        .map(|code| {
            tagged(
                session,
                slot,
                DeviceClass::Keyboard,
                InputKind::Key {
                    code: KeyCode(code),
                    edge: Edge::Release,
                },
            )
        })
        .chain(buttons.into_iter().map(|code| {
            tagged(
                session,
                slot,
                DeviceClass::Pointer,
                InputKind::Button {
                    button: ButtonCode(code),
                    edge: Edge::Release,
                },
            )
        }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_count_up_and_run_out_instead_of_repeating() {
        let (mint, first) = RemoteSlots::default().next().expect("a slot");
        let (_, second) = mint.next().expect("a slot");
        assert_eq!((first.number(), second.number()), (0, 1));
        let last = RemoteSlots {
            next: MAX_REMOTE_SLOTS - 1,
        };
        let (spent, slot) = last.next().expect("the last slot");
        assert_eq!(slot.number(), MAX_REMOTE_SLOTS - 1);
        assert_eq!(spent.next(), None, "no slot is handed out twice");
    }

    /// CASW-19: the device id comes from the slot, whatever the portal's session id is, so two
    /// sessions that differ only above bit 28 (or are the same session twice) never collide.
    #[test]
    fn two_connections_never_share_a_device_id() {
        let classes = [
            DeviceClass::Keyboard,
            DeviceClass::Pointer,
            DeviceClass::Touch,
        ];
        let mut seen = BTreeSet::new();
        let mut mint = RemoteSlots::default();
        for _ in 0..4 {
            let (next, slot) = mint.next().expect("a slot");
            mint = next;
            for class in classes {
                let id = device_id(slot, class);
                assert!(id.0 & 0x8000_0000 != 0, "clear of the real backends' ids");
                assert!(seen.insert(id.0), "{id:?} was handed out twice");
            }
        }
    }

    #[test]
    fn a_full_held_set_drops_new_presses_and_releases_empty_it() {
        let held = Held::default();
        let cap = u32::try_from(MAX_HELD).expect("a small cap");
        for code in 0..cap {
            assert_eq!(held.key(code, Edge::Press), Fit::Held);
        }
        assert_eq!(held.key(cap, Edge::Press), Fit::Full);
        assert_eq!(
            held.key(3, Edge::Press),
            Fit::Held,
            "a key already held fits"
        );
        assert_eq!(held.key(3, Edge::Release), Fit::Held);
        assert_eq!(held.key(cap, Edge::Press), Fit::Held, "room again");
    }

    #[test]
    fn a_handle_releases_what_is_held_once() {
        let held = Held::default();
        held.key(30, Edge::Press);
        held.button(0x110, Edge::Press);
        let handle = RemoteHandle::new(PortalSessionId(4), RemoteSlot(2), held.clone());
        let kinds: Vec<InputKind> = handle
            .release()
            .into_iter()
            .map(|input| input.kind)
            .collect();
        assert_eq!(
            kinds,
            vec![
                InputKind::Key {
                    code: KeyCode(30),
                    edge: Edge::Release
                },
                InputKind::Button {
                    button: ButtonCode(0x110),
                    edge: Edge::Release
                },
            ]
        );
        assert!(
            handle.release().is_empty(),
            "a second release finds nothing"
        );
        assert!(releases(PortalSessionId(4), RemoteSlot(2), &held).is_empty());
    }
}
