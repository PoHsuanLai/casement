//! Reach, chords, binding resolution and the I7 table (what P0-F1-input makes pass).

mod support;

use casement_core::input::{
    Binding, BindingAction, Bindings, Chord, DesktopCommand, DeviceClass, FeedSource, InputTarget,
    InputUse, Keysym, Origin, PortalSessionId, Reach, ReachFacts, Refusal, Resolved, reach,
    resolve,
};
use casement_core::lock::{LockState, LockSurfaces};
use casement_core::policy::{Advertise, ConnectionRole, GlobalKind, advertised};
use casement_core::{LayerKey, LeaseId, WindowId};
use casement_protocols::vocab::MacMods;
use porter_core::SandboxFacts;

const KEYBOARD: Origin = Origin::Physical(DeviceClass::Keyboard);
const POINTER: Origin = Origin::Physical(DeviceClass::Pointer);
const VIRTUAL: Origin = Origin::Virtual(DeviceClass::Keyboard);
const FEED: Origin = Origin::Feed(FeedSource::Palmrest);
const WINDOW: InputTarget = InputTarget::Window(WindowId(1));

#[test]
fn the_reach_table() {
    let unlocked = LockState::Unlocked;
    let locked = LockState::Locked {
        surfaces: LockSurfaces::Pending,
    };
    let cases: Vec<(&str, Origin, InputTarget, InputUse, &LockState, Reach)> = vec![
        (
            "physical key to a window",
            KEYBOARD,
            WINDOW,
            InputUse::Key,
            &unlocked,
            Reach::Allowed,
        ),
        (
            "physical press on a layer",
            POINTER,
            InputTarget::Layer(LayerKey(1)),
            InputUse::Press,
            &unlocked,
            Reach::Allowed,
        ),
        (
            "feed scrolls",
            FEED,
            WINDOW,
            InputUse::Scroll,
            &unlocked,
            Reach::Allowed,
        ),
        (
            "feed gestures",
            FEED,
            WINDOW,
            InputUse::SystemGesture,
            &unlocked,
            Reach::Allowed,
        ),
        (
            "feed may not press",
            FEED,
            WINDOW,
            InputUse::Press,
            &unlocked,
            Reach::Refused(Refusal::FeedKind),
        ),
        (
            "feed may not type",
            FEED,
            WINDOW,
            InputUse::Key,
            &unlocked,
            Reach::Refused(Refusal::FeedKind),
        ),
        (
            "remote key",
            Origin::Remote(PortalSessionId(1)),
            WINDOW,
            InputUse::Key,
            &unlocked,
            Reach::Allowed,
        ),
        (
            "virtual key",
            VIRTUAL,
            WINDOW,
            InputUse::Key,
            &unlocked,
            Reach::Allowed,
        ),
        (
            "virtual press on a trusted surface",
            VIRTUAL,
            InputTarget::Trusted,
            InputUse::Press,
            &unlocked,
            Reach::Refused(Refusal::NotPhysical),
        ),
        (
            "remote press on a trusted surface",
            Origin::Remote(PortalSessionId(1)),
            InputTarget::Trusted,
            InputUse::Press,
            &unlocked,
            Reach::Refused(Refusal::NotPhysical),
        ),
        (
            "physical press on a trusted surface",
            POINTER,
            InputTarget::Trusted,
            InputUse::Press,
            &unlocked,
            Reach::Allowed,
        ),
        (
            "agent before leases exist",
            Origin::Agent(LeaseId(1)),
            WINDOW,
            InputUse::Press,
            &unlocked,
            Reach::Refused(Refusal::OutsideLease),
        ),
        (
            "physical key to the lock surface",
            KEYBOARD,
            InputTarget::LockSurface,
            InputUse::Key,
            &locked,
            Reach::Allowed,
        ),
        (
            "test key to the lock surface",
            Origin::Test,
            InputTarget::LockSurface,
            InputUse::Key,
            &locked,
            Reach::Allowed,
        ),
        (
            "physical press on a window while locked",
            POINTER,
            WINDOW,
            InputUse::Press,
            &locked,
            Reach::Refused(Refusal::Locked),
        ),
        (
            "virtual key to the lock surface",
            VIRTUAL,
            InputTarget::LockSurface,
            InputUse::Key,
            &locked,
            Reach::Refused(Refusal::Locked),
        ),
        (
            "feed scroll while locked",
            FEED,
            WINDOW,
            InputUse::Scroll,
            &LockState::LockedWithoutSurface,
            Reach::Refused(Refusal::Locked),
        ),
    ];
    for (name, origin, target, use_, lock, want) in cases {
        assert_eq!(
            reach(&origin, &target, use_, &ReachFacts { lock }),
            want,
            "{name}"
        );
    }
}

/// A text, the chord it parses to (modifiers, keysym), its printed form.
type ChordCase = (&'static str, Option<(MacMods, u32)>, &'static str);

#[test]
fn chords_parse_in_any_order_and_print_in_the_macs() {
    let cases: [ChordCase; 7] = [
        ("cmd+space", Some((MacMods::COMMAND, 0x20)), "cmd+space"),
        (
            "shift+cmd+4",
            Some((MacMods::COMMAND.with(MacMods::SHIFT), 0x34)),
            "shift+cmd+4",
        ),
        (
            "cmd+shift+4",
            Some((MacMods::COMMAND.with(MacMods::SHIFT), 0x34)),
            "shift+cmd+4",
        ),
        (
            "option+ctrl+t",
            Some((MacMods::CONTROL.with(MacMods::OPTION), 0x74)),
            "ctrl+option+t",
        ),
        ("cmd+Tab", Some((MacMods::COMMAND, 0xff09)), "cmd+Tab"),
        ("cmd+", None, ""),
        ("hyper+a", None, ""),
    ];
    for (text, want, printed) in cases {
        let got = Chord::parse(text).ok();
        assert_eq!(got.map(|c| (c.mods, c.key.0)), want, "{text}");
        if let Some(chord) = got {
            assert_eq!(chord.text(), printed, "{text}");
            assert_eq!(Chord::parse(&chord.text()), Ok(chord), "{text} round trip");
        }
    }
    assert!(Chord::parse("").is_err());
    // Texts that only need to parse, or not (names, case, repeated or missing parts).
    const PARSES: &[(&str, bool)] = &[
        ("cmd+plus", true),
        ("ctrl+option+shift+cmd+fn+F12", true),
        ("Cmd+SHIFT+a", true),
        ("space", true),
        ("cmd+cmd+a", false),
        ("+a", false),
        ("cmd++a", false),
        ("cmd+shift", false),
        ("cmd", false),
        ("cmd+a+", false),
    ];
    for (text, parses) in PARSES {
        assert_eq!(Chord::parse(text).is_ok(), *parses, "{text:?}");
    }
}

#[test]
fn agent_remote_and_feed_never_fire_a_binding() {
    let hide = Chord {
        mods: MacMods::COMMAND,
        key: Keysym(0x68),
    };
    let quit = Chord {
        mods: MacMods::COMMAND,
        key: Keysym(0x71),
    };
    let bindings = Bindings(vec![
        Binding {
            chord: hide,
            action: BindingAction::Desktop(DesktopCommand::HideActive),
        },
        Binding {
            chord: hide,
            action: BindingAction::Desktop(DesktopCommand::HideOthers),
        },
    ]);
    let fires = Resolved::Action(BindingAction::Desktop(DesktopCommand::HideActive));
    let cases = [
        ("physical, first match wins", KEYBOARD, hide, fires.clone()),
        ("virtual keyboard", VIRTUAL, hide, fires.clone()),
        ("test", Origin::Test, hide, fires),
        ("unbound chord", KEYBOARD, quit, Resolved::PassThrough),
        (
            "agent",
            Origin::Agent(LeaseId(1)),
            hide,
            Resolved::PassThrough,
        ),
        (
            "remote",
            Origin::Remote(PortalSessionId(2)),
            hide,
            Resolved::PassThrough,
        ),
        ("feed", FEED, hide, Resolved::PassThrough),
    ];
    for (name, origin, chord, want) in cases {
        assert_eq!(resolve(&bindings, &chord, &origin), want, "{name}");
    }
}

/// The globals a sandboxed client never sees.
const HIDDEN_FROM_SANDBOXES: &[GlobalKind] = &[
    GlobalKind::ExtDataControl,
    GlobalKind::WlrDataControl,
    GlobalKind::ForeignToplevelList,
    GlobalKind::InputMethod,
    GlobalKind::VirtualKeyboard,
    GlobalKind::SecurityContext,
    GlobalKind::LayerShell,
    GlobalKind::SessionLock,
];

#[test]
fn the_i7_table() {
    let sandbox = ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    });
    let every = [
        GlobalKind::Compositor,
        GlobalKind::Shm,
        GlobalKind::XdgShell,
        GlobalKind::QuireWindow,
        GlobalKind::Seat,
        GlobalKind::Dmabuf,
        GlobalKind::ExtDataControl,
        GlobalKind::WlrDataControl,
        GlobalKind::ForeignToplevelList,
        GlobalKind::InputMethod,
        GlobalKind::VirtualKeyboard,
        GlobalKind::SecurityContext,
        GlobalKind::LayerShell,
        GlobalKind::SessionLock,
    ];
    for global in every {
        for role in [
            ConnectionRole::Public,
            ConnectionRole::Shell,
            ConnectionRole::Agent,
        ] {
            assert_eq!(
                advertised(global, &role),
                Advertise::Yes,
                "{global:?} on {role:?}"
            );
        }
        let want = if HIDDEN_FROM_SANDBOXES.contains(&global) {
            Advertise::No
        } else {
            Advertise::Yes
        };
        assert_eq!(
            advertised(global, &sandbox),
            want,
            "{global:?} in a sandbox"
        );
    }
}
