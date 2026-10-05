//! `virtual_keyboard` (P0-H7).

use casement_core::policy::ConnectionRole;
use casement_core::time::Mono;
use casement_testclient::ime::virtual_keyboard::VirtualBoard;
use casement_testclient::misc::idle::{IdleKind, IdleLine, IdleSignal, IdleWatch};
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;
use std::time::Duration;

fn sees_virtual_keyboard(fixture: &mut Fixture, role: ConnectionRole) -> bool {
    let client = fixture.connect(role);
    fixture
        .client(client)
        .globals()
        .contents()
        .with_list(|list| {
            list.iter().any(|global| {
                global.interface == "zwp_virtual_keyboard_manager_v1" && global.version == 1
            })
        })
}

/// A Public connection sees zwp_virtual_keyboard_manager_v1 (version 1); a Sandboxed one
/// does not (I7).
#[test]
fn the_virtual_keyboard_manager_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    assert!(sees_virtual_keyboard(&mut fixture, ConnectionRole::Public));
    let sandboxed = ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    });
    assert!(!sees_virtual_keyboard(&mut fixture, sandboxed));
}

/// The smallest keymap libxkbcommon accepts: one key.
const KEYMAP: &str = "xkb_keymap {\n xkb_keycodes { <A> = 38; };\n xkb_types {};\n xkb_compat {};\n xkb_symbols { key <A> { [ a ] }; };\n};\n";

/// A virtual keyboard's key resets idleness (I8: every origin counts): an idle notification of
/// 30 s asked at 0 goes off at 50 s when a virtual key arrives at 20 s. A `key` before the
/// keymap is the protocol error `no_keymap`. (That the model sees the key as
/// `Origin::Virtual(Keyboard)` is the unit test of `casement_wayland::virtual_keyboard`.)
#[test]
fn virtual_keys_are_origin_virtual_and_count_as_activity() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let keys = fixture.connect(ConnectionRole::Public);
    let watcher = fixture.connect(ConnectionRole::Public);
    let careless = fixture.connect(ConnectionRole::Public);
    fixture.pump();

    let early = VirtualBoard::start(fixture.client(careless).globals()).expect("the keyboard");
    fixture.pump();
    early.key(0, 30, 1).expect("key");
    fixture.pump();
    let error = fixture
        .client(careless)
        .protocol_error()
        .expect("a key before the keymap is an error");
    assert_eq!(error.interface, "zwp_virtual_keyboard_v1");
    assert_eq!(error.code, 0, "no_keymap");

    let mut watch = IdleWatch::start(
        fixture.client(watcher).globals(),
        IdleKind::Inhibitable,
        Duration::from_secs(30),
    )
    .expect("the notification");
    let board = VirtualBoard::start(fixture.client(keys).globals()).expect("the keyboard");
    fixture.pump();
    board.keymap(KEYMAP).expect("keymap");
    fixture.pump();
    let mut step = |fixture: &mut Fixture, by: Duration| {
        fixture.advance(by);
        watch.pump(fixture.model().now()).expect("pump the watch");
        watch.lines().to_vec()
    };
    assert!(step(&mut fixture, Duration::from_secs(20)).is_empty());
    board.key(0, 30, 1).expect("key");
    board.key(0, 30, 0).expect("key");
    assert!(
        step(&mut fixture, Duration::from_millis(29_999)).is_empty(),
        "the key at 20 s moved the deadline to 50 s"
    );
    assert!(
        fixture.client(keys).protocol_error().is_none(),
        "a key after the keymap is fine"
    );
    let at_fifty = step(&mut fixture, Duration::from_millis(1));
    assert_eq!(
        at_fifty,
        [IdleLine {
            at: Mono(Duration::from_secs(50)),
            signal: IdleSignal::Idled
        }]
    );
}
