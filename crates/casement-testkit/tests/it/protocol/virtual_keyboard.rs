//! `virtual_keyboard` (P0-H7).

use super::consent::{ScriptedShell, asked_world, kept, program};
use casement_core::Event;
use casement_core::policy::ConnectionRole;
use casement_core::privilege::{HELD_REQUESTS, UnixSeconds, Verdict};
use casement_core::time::Mono;
use casement_protocols::vocab::{Answer, Power};
use casement_testclient::ime::virtual_keyboard::VirtualBoard;
use casement_testclient::misc::idle::{IdleKind, IdleLine, IdleSignal, IdleWatch};
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testkit::{ASKED_PROGRAM, Fixture, FixtureSpec};
use std::time::Duration;

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

/// CASW-09: a virtual keyboard may send `MAX_KEYMAPS` keymaps (each is compiled by libxkbcommon
/// on the loop); the next one cuts its client with `wl_display.no_memory` (code 2).
#[test]
fn a_keyboard_that_sends_keymaps_without_end_is_cut() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let board = VirtualBoard::start(fixture.client(id).globals()).expect("the keyboard");
    fixture.pump();
    for _ in 0..casement_wayland::limits::MAX_KEYMAPS {
        board.keymap(KEYMAP).expect("keymap");
    }
    fixture.pump();
    assert!(
        fixture.client(id).protocol_error().is_none(),
        "up to the cap is allowed"
    );
    board.keymap(KEYMAP).expect("keymap");
    fixture.pump();
    // The cut closes the connection (the system backend sends no error first).
    assert!(
        !fixture.client(id).is_connected(),
        "the client past the cap is cut"
    );
}

/// The shell holds the keyboard on an exclusive launcher, so the keys any keyboard types reach
/// it. Returns what keeps the focus alive.
fn listening(fixture: &mut Fixture, shell: &ScriptedShell) -> (Keyboard, LayerSurface) {
    let keyboard = Keyboard::bind(fixture.client(shell.id)).expect("a keyboard");
    let launcher = LayerSurface::create(
        fixture.client(shell.id),
        &LayerAsk {
            level: Layer::Overlay,
            namespace: "launcher",
            anchor: Anchor::empty(),
            exclusive_zone: 0,
            keyboard: KeyboardInteractivity::Exclusive,
            size: (400, 300),
        },
    )
    .expect("layer");
    fixture.pump();
    (keyboard, launcher)
}

/// How many `key` events the shell's keyboard has heard.
fn keys_heard(fixture: &mut Fixture, shell: &ScriptedShell) -> usize {
    fixture
        .client(shell.id)
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with("wl_keyboard#") && line.event == "key")
        .count()
}

/// A keyboard made by a program that is asked: the shell is asked, and the keymap and the keys
/// typed meanwhile wait; they are replayed in order when the person allows.
#[test]
fn keys_typed_while_the_question_is_open_are_replayed_on_allow() {
    let (mut fixture, shell, client) = asked_world();
    let _focus = listening(&mut fixture, &shell);
    let board = VirtualBoard::start(fixture.client(client).globals()).expect("the keyboard");
    board.keymap(KEYMAP).expect("keymap");
    board.key(0, 30, 1).expect("key");
    board.key(0, 30, 0).expect("key");
    fixture.pump();
    assert_eq!(shell.asked().len(), 1, "the shell is asked");
    assert_eq!(keys_heard(&mut fixture, &shell), 0, "nothing is typed yet");
    assert!(fixture.client(client).protocol_error().is_none());

    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    assert_eq!(
        keys_heard(&mut fixture, &shell),
        2,
        "the press and the release"
    );
    board.key(0, 31, 1).expect("key");
    fixture.pump();
    assert_eq!(
        keys_heard(&mut fixture, &shell),
        3,
        "and now keys go straight on"
    );
}

/// Don't Allow kills the keyboard: what it held is dropped, and nothing it types reaches anyone,
/// without an error to the program.
#[test]
fn a_refused_keyboard_types_nothing() {
    let (mut fixture, shell, client) = asked_world();
    let _focus = listening(&mut fixture, &shell);
    let board = VirtualBoard::start(fixture.client(client).globals()).expect("the keyboard");
    board.keymap(KEYMAP).expect("keymap");
    board.key(0, 30, 1).expect("key");
    fixture.pump();
    shell.answer(&mut fixture, shell.only_ask(), Answer::Deny);
    board.key(0, 30, 0).expect("key");
    board.key(0, 31, 1).expect("key");
    fixture.pump();
    assert_eq!(keys_heard(&mut fixture, &shell), 0);
    assert!(fixture.client(client).protocol_error().is_none());
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Keys),
        Some(Verdict::Deny)
    );
}

/// Taking the power back in Settings ends the keyboard at once: the key it held down is
/// released, and what it types after that goes nowhere.
#[test]
fn a_power_taken_back_releases_the_held_keys_and_ends_the_keyboard() {
    let (mut fixture, shell, client) = asked_world();
    let _focus = listening(&mut fixture, &shell);
    let board = VirtualBoard::start(fixture.client(client).globals()).expect("the keyboard");
    board.keymap(KEYMAP).expect("keymap");
    fixture.pump();
    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    board.key(0, 30, 1).expect("key");
    fixture.pump();
    assert_eq!(keys_heard(&mut fixture, &shell), 1);

    fixture.feed_events(vec![Event::PrivilegeRevoked {
        subject: program(ASKED_PROGRAM),
        power: Power::Keys,
        at: UnixSeconds(2),
    }]);
    assert_eq!(keys_heard(&mut fixture, &shell), 2, "the key is let go");
    board.key(0, 31, 1).expect("key");
    fixture.pump();
    assert_eq!(
        keys_heard(&mut fixture, &shell),
        2,
        "and the keyboard is dead"
    );
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Keys),
        Some(Verdict::Deny)
    );
}

/// A keyboard holds at most `HELD_REQUESTS` requests while the question is open (its keymap
/// counts as one); the rest are dropped, so a program cannot grow the compositor by typing.
#[test]
fn a_keyboard_holds_at_most_the_cap_of_requests_while_asked() {
    let (mut fixture, shell, client) = asked_world();
    let _focus = listening(&mut fixture, &shell);
    let board = VirtualBoard::start(fixture.client(client).globals()).expect("the keyboard");
    board.keymap(KEYMAP).expect("keymap");
    // The compositor reads as the keys are sent, as it would: the socket holds only so much.
    for _ in 0..HELD_REQUESTS {
        board.key(0, 30, 1).expect("key");
        board.key(0, 30, 0).expect("key");
        fixture.pump();
    }
    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    assert_eq!(
        keys_heard(&mut fixture, &shell),
        HELD_REQUESTS - 1,
        "the keymap and the keys that fit"
    );
}
