//! Properties over every origin, target and use (the tables in `tests/input.rs` pin examples),
//! and the keysym names behind the chord text.

use super::{
    Binding, BindingAction, Bindings, Chord, DesktopCommand, DeviceClass, FeedSource, InputTarget,
    InputUse, Keysym, Origin, PortalSessionId, Reach, ReachFacts, Refusal, Resolved, reach,
    resolve,
};
use crate::ids::{ClientKey, LayerKey, LeaseId, WindowId};
use crate::lock::{LockHolder, LockState, LockSurfaces};
use crate::time::Mono;
use casement_protocols::vocab::MacMods;
use proptest::prelude::*;
use std::time::Duration;

fn device_class() -> impl Strategy<Value = DeviceClass> {
    prop_oneof![
        Just(DeviceClass::Keyboard),
        Just(DeviceClass::Pointer),
        Just(DeviceClass::Touchpad),
        Just(DeviceClass::Tablet),
        Just(DeviceClass::Touch),
        Just(DeviceClass::Switch),
    ]
}

fn origin() -> impl Strategy<Value = Origin> {
    prop_oneof![
        device_class().prop_map(Origin::Physical),
        device_class().prop_map(Origin::Virtual),
        Just(Origin::Feed(FeedSource::Palmrest)),
        any::<u64>().prop_map(|n| Origin::Agent(LeaseId(n))),
        any::<u64>().prop_map(|n| Origin::Remote(PortalSessionId(n))),
        Just(Origin::Test),
    ]
}

fn target() -> impl Strategy<Value = InputTarget> {
    prop_oneof![
        any::<u64>().prop_map(|n| InputTarget::Window(WindowId(n))),
        any::<u64>().prop_map(|n| InputTarget::Popup(WindowId(n))),
        any::<u64>().prop_map(|n| InputTarget::Layer(LayerKey(n))),
        Just(InputTarget::LockSurface),
        Just(InputTarget::Trusted),
        Just(InputTarget::Nothing),
    ]
}

fn use_() -> impl Strategy<Value = InputUse> {
    prop_oneof![
        Just(InputUse::Key),
        Just(InputUse::Press),
        Just(InputUse::Motion),
        Just(InputUse::Scroll),
        Just(InputUse::SystemGesture),
        Just(InputUse::Text),
    ]
}

fn lock() -> impl Strategy<Value = LockState> {
    prop_oneof![
        Just(LockState::Unlocked),
        Just(LockState::Locking {
            since: Mono(Duration::from_secs(1)),
            holder: LockHolder::Client(ClientKey(1)),
        }),
        Just(LockState::Locked {
            holder: LockHolder::Client(ClientKey(1)),
            surfaces: LockSurfaces::Pending,
        }),
        Just(LockState::Locked {
            holder: LockHolder::Client(ClientKey(1)),
            surfaces: LockSurfaces::Covering(Vec::new()),
        }),
        Just(LockState::LockedWithoutSurface {
            holder: LockHolder::Client(ClientKey(1))
        }),
    ]
}

fn is_locked(lock: &LockState) -> bool {
    !matches!(lock, LockState::Unlocked)
}

proptest! {
    #[test]
    fn the_agent_never_reaches_anything_before_leases_exist(
        lease in any::<u64>(), target in target(), use_ in use_(), lock in lock(),
    ) {
        let got = reach(&Origin::Agent(LeaseId(lease)), &target, use_, &ReachFacts { lock: &lock });
        prop_assert_ne!(got, Reach::Allowed);
    }

    #[test]
    fn a_trusted_surface_only_hears_physical_input(
        origin in origin(), use_ in use_(), lock in lock(),
    ) {
        let got = reach(&origin, &InputTarget::Trusted, use_, &ReachFacts { lock: &lock });
        prop_assert!(got != Reach::Allowed || (origin.is_physical() && !is_locked(&lock)));
    }

    #[test]
    fn a_locked_session_lets_only_physical_input_reach_the_lock_surface(
        origin in origin(), target in target(), use_ in use_(), lock in lock(),
    ) {
        let got = reach(&origin, &target, use_, &ReachFacts { lock: &lock });
        if is_locked(&lock) && got == Reach::Allowed {
            prop_assert!(origin.is_physical());
            prop_assert_eq!(target, InputTarget::LockSurface);
        }
        if is_locked(&lock) && !(origin.is_physical() && target == InputTarget::LockSurface) {
            prop_assert_eq!(got, Reach::Refused(Refusal::Locked));
        }
    }

    #[test]
    fn a_feed_reaches_only_by_scroll_and_gesture(
        target in target(), use_ in use_(), lock in lock(),
    ) {
        let feed = Origin::Feed(FeedSource::Palmrest);
        let got = reach(&feed, &target, use_, &ReachFacts { lock: &lock });
        if got == Reach::Allowed {
            prop_assert!(matches!(use_, InputUse::Scroll | InputUse::SystemGesture));
        }
    }

    #[test]
    fn an_unlocked_physical_event_reaches_every_target(
        class in device_class(), target in target(), use_ in use_(),
    ) {
        let got = reach(&Origin::Physical(class), &target, use_, &ReachFacts { lock: &LockState::Unlocked });
        prop_assert_eq!(got, Reach::Allowed);
    }

    #[test]
    fn only_the_persons_hands_and_virtual_keyboards_fire_bindings(
        origin in origin(), mods in 0u32..64, key in any::<u32>(),
    ) {
        let chord = Chord { mods: MacMods(mods), key: Keysym(key) };
        let bindings = Bindings(vec![Binding {
            chord,
            action: BindingAction::Desktop(DesktopCommand::QuitActive),
        }]);
        let want = if origin.fires_bindings() {
            Resolved::Action(BindingAction::Desktop(DesktopCommand::QuitActive))
        } else {
            Resolved::PassThrough
        };
        prop_assert_eq!(resolve(&bindings, &chord, &origin), want);
    }

    #[test]
    fn only_a_kernel_keyboard_switches_the_vt(origin in origin()) {
        prop_assert_eq!(origin.switches_vt(), matches!(origin, Origin::Physical(_)));
    }

    #[test]
    fn every_keysym_reads_back_from_its_name(code in any::<u32>()) {
        let keysym = Keysym(code);
        prop_assert_eq!(Keysym::named(&keysym.name()), Some(keysym));
    }

    #[test]
    fn every_chord_reads_back_from_its_text(mods in 0u32..64, key in any::<u32>()) {
        let held = MacMods(mods & !MacMods::CAPS_LOCK.0);
        let chord = Chord { mods: held, key: Keysym(key) };
        prop_assert_eq!(Chord::parse(&chord.text()), Ok(chord));
    }
}

/// A text, the keysym it names.
const NAMES: &[(&str, Option<u32>)] = &[
    ("a", Some(0x61)),
    ("A", Some(0x41)),
    ("0", Some(0x30)),
    ("space", Some(0x20)),
    ("plus", Some(0x2b)),
    ("grave", Some(0x60)),
    ("Return", Some(0xff0d)),
    ("Page_Down", Some(0xff56)),
    ("F1", Some(0xffbe)),
    ("F24", Some(0xffd5)),
    ("F0", None),
    ("F25", None),
    ("F05", None),
    ("U20AC", Some(0x0100_20ac)),
    ("U20ac", Some(0x0100_20ac)),
    ("U0041", None),
    ("0xff09", Some(0xff09)),
    ("0x", None),
    ("0x+5", None),
    ("+", None),
    ("Hyper_L", None),
    ("", None),
    ("ab", None),
];

#[test]
fn keysym_names_table() {
    for (text, want) in NAMES {
        assert_eq!(Keysym::named(text), want.map(Keysym), "{text:?}");
    }
}

#[test]
fn caps_lock_is_not_part_of_a_chord_text() {
    let chord = Chord {
        mods: MacMods::COMMAND.with(MacMods::CAPS_LOCK),
        key: Keysym(0x61),
    };
    assert_eq!(chord.text(), "cmd+a");
}
