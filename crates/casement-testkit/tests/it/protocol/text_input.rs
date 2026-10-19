//! `text_input` (P0-H7).

use crate::protocol::xdg_shell::{
    boot, check_golden, click, golden, map_toplevel, pointer, settle, windows,
};
use casement_core::DeviceId;
use casement_core::input::{DeviceClass, Edge, KeyCode};
use casement_testclient::ime::input_method::InputMethodEvent;
use casement_testclient::ime::session::ImeSession;
use casement_testclient::ime::text_input::TextInputEvent;
use casement_testclient::ime::virtual_keyboard::VirtualBoard;
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testkit::{ClientId, Fixture, TestOrigin};

/// The smallest keymap libxkbcommon accepts: one key.
const KEYMAP: &str = "xkb_keymap {\n xkb_keycodes { <A> = 38; };\n xkb_types {};\n xkb_compat {};\n xkb_symbols { key <A> { [ a ] }; };\n};\n";

/// Whether the input method is bound when the session starts.
#[derive(Debug, Clone, Copy)]
enum Method {
    /// It binds with the field.
    Bound,
    /// It starts later (fcitx5 after the apps).
    Later,
}

/// A document window with keyboard focus, a text field and an input method on its connection.
pub(crate) struct Typing {
    pub(crate) fixture: Fixture,
    pub(crate) client: ClientId,
    pub(crate) ime: ImeSession,
    pub(crate) keyboard: DeviceId,
    _window: Toplevel,
}

impl Typing {
    /// Maps a toplevel, binds the field and the method (before the window is focused, so the
    /// field gets `enter` with the focus), then clicks the window: it takes the key.
    pub(crate) fn start() -> Typing {
        Typing::start_with(true)
    }

    /// As [`Typing::start`], but the input method binds only if `with_method` (an IME that
    /// starts after the apps binds later with [`Typing::start_method`]).
    pub(crate) fn start_with(with_method: bool) -> Typing {
        let (fixture, client) = boot();
        let method = match with_method {
            true => Method::Bound,
            false => Method::Later,
        };
        Typing::in_world(fixture, client, method)
    }

    /// As [`Typing::start`], in a world the caller built: `client` is connected to `fixture`
    /// and is the one that maps the document and binds the field and the method (a program that
    /// is asked about its powers, for the consent tests).
    pub(crate) fn start_in(fixture: Fixture, client: ClientId) -> Typing {
        Typing::in_world(fixture, client, Method::Bound)
    }

    fn in_world(mut fixture: Fixture, client: ClientId, method: Method) -> Typing {
        let base = WmBase::bind(fixture.client(client), 7);
        let window = map_toplevel(&mut fixture, client, &base, (400, 300));
        let globals = fixture.client(client).globals();
        let ime = match method {
            Method::Bound => ImeSession::start(globals),
            Method::Later => ImeSession::start_field(globals),
        }
        .expect("the session");
        fixture.pump();
        settle(&mut fixture);
        let id = windows(&fixture)[0];
        let device = pointer(&mut fixture);
        click(&mut fixture, device, id);
        let keyboard = fixture.input().plug(
            DeviceClass::Keyboard,
            TestOrigin::Physical(DeviceClass::Keyboard),
        );
        let mut typing = Typing {
            fixture,
            client,
            ime,
            keyboard,
            _window: window,
        };
        typing.pump();
        assert_eq!(
            typing.fixture.model().focus().key,
            Some(id),
            "the click focused the document"
        );
        typing
    }

    /// The compositor dispatches, then the field and the method read what arrived.
    pub(crate) fn pump(&mut self) {
        self.fixture.pump();
        let now = self.fixture.model().now();
        self.ime.pump(now).expect("pump the session");
    }

    /// The input method starts (fcitx5 after the apps): it binds, and the compositor answers.
    pub(crate) fn start_method(&mut self) {
        let globals = self.fixture.client(self.client).globals();
        self.ime.bind_method(globals).expect("bind the method");
        self.pump();
        self.pump();
    }

    /// The field enables text input; the method hears `activate` and grabs the keyboard.
    pub(crate) fn enable_and_grab(&mut self) {
        self.ime.enable().expect("enable");
        self.pump();
        assert!(
            self.ime
                .method_events()
                .iter()
                .any(|line| line.event == InputMethodEvent::Activate),
            "the method is activated on the enabled field"
        );
        self.ime.grab_keyboard().expect("grab");
        self.pump();
    }
}

/// Enter, enable, then the first key goes to the input method, not the document (SP1 A5).
///
/// The key arrives after the field's `enable` and before the method has read `activate`, so
/// the keyboard is not grabbed yet: the key waits at the gate, and goes to the grab once the
/// method takes it. The document's own `wl_keyboard` sees none.
#[test]
fn the_first_key_after_enable_reaches_the_input_method() {
    let mut typing = Typing::start();
    assert!(
        typing
            .ime
            .field_events()
            .iter()
            .any(|line| line.event == TextInputEvent::Enter),
        "the field got enter with the focus"
    );
    typing.ime.enable().expect("enable");
    typing.fixture.pump();
    let keyboard = typing.keyboard;
    typing
        .fixture
        .input()
        .key(keyboard, KeyCode(30), Edge::Press);
    // The method reads `activate` only now, and answers with its grab.
    let now = typing.fixture.model().now();
    typing.ime.pump(now).expect("pump the session");
    assert!(
        typing
            .ime
            .method_events()
            .iter()
            .all(|line| !matches!(line.event, InputMethodEvent::GrabKey { .. })),
        "the key waited: no grab yet"
    );
    typing.ime.grab_keyboard().expect("grab");
    typing.pump();
    typing.pump();
    let taken: Vec<_> = typing
        .ime
        .method_events()
        .iter()
        .filter_map(|line| match line.event {
            InputMethodEvent::GrabKey { key, state } => Some((key, state)),
            _ => None,
        })
        .collect();
    assert_eq!(taken, vec![(30, 1)], "the grab saw the key");
    assert!(
        typing.ime.document_keys().is_empty(),
        "the document saw no key"
    );
    typing
        .fixture
        .input()
        .key(keyboard, KeyCode(30), Edge::Release);
    typing.pump();
    assert!(
        typing.ime.document_keys().is_empty(),
        "the release goes to the method too"
    );
}

/// An input method that holds the keyboard sends a key it does not use back through its own
/// virtual keyboard while the person still holds the real one (fcitx5 in English mode, the
/// launcher on the trial login of 2026-10-19). The document gets that key and its release; the
/// method gets the real key and its release, and nothing twice.
#[test]
fn a_key_the_input_method_sends_back_reaches_the_document() {
    let mut typing = Typing::start();
    typing.enable_and_grab();
    let board =
        VirtualBoard::start(typing.fixture.client(typing.client).globals()).expect("the board");
    typing.pump();
    board.keymap(KEYMAP).expect("keymap");
    typing.pump();
    let keyboard = typing.keyboard;
    typing
        .fixture
        .input()
        .key(keyboard, KeyCode(30), Edge::Press);
    typing.pump();
    board.key(0, 30, 1).expect("the key back");
    typing.pump();
    typing
        .fixture
        .input()
        .key(keyboard, KeyCode(30), Edge::Release);
    typing.pump();
    board.key(0, 30, 0).expect("the release back");
    typing.pump();
    let grabbed: Vec<_> = typing
        .ime
        .method_events()
        .iter()
        .filter_map(|line| match line.event {
            InputMethodEvent::GrabKey { key, state } => Some((key, state)),
            _ => None,
        })
        .collect();
    assert_eq!(
        grabbed,
        vec![(30, 1), (30, 0)],
        "the method had the real key"
    );
    assert_eq!(
        typing
            .ime
            .document_keys()
            .iter()
            .map(|key| (key.key, key.state))
            .collect::<Vec<_>>(),
        vec![(30, 1), (30, 0)],
        "the document had the key sent back"
    );
}

/// The grab ends between the copy's press and its release: the release still
/// reaches the document the press went to, and the next key reaches it through the keyboard, so
/// no key is left held on either side.
#[test]
fn a_copy_released_after_the_grab_ends_leaves_no_key_held() {
    let mut typing = Typing::start();
    typing.enable_and_grab();
    let board =
        VirtualBoard::start(typing.fixture.client(typing.client).globals()).expect("the board");
    typing.pump();
    board.keymap(KEYMAP).expect("keymap");
    typing.pump();
    let keyboard = typing.keyboard;
    typing
        .fixture
        .input()
        .key(keyboard, KeyCode(30), Edge::Press);
    typing.pump();
    board.key(0, 30, 1).expect("the key back");
    typing.pump();
    typing.ime.release_grab().expect("the method lets go");
    typing.pump();
    board.key(0, 30, 0).expect("the release back");
    typing.pump();
    typing
        .fixture
        .input()
        .key(keyboard, KeyCode(30), Edge::Release);
    typing.pump();
    // The field is still enabled, so the key waits for a method first (the hold limit).
    for edge in [Edge::Press, Edge::Release] {
        typing.fixture.input().key(keyboard, KeyCode(30), edge);
        typing.pump();
        typing
            .fixture
            .advance(casement_wayland::text_input::HOLD_LIMIT);
        typing.pump();
    }
    let keys: Vec<_> = typing
        .ime
        .document_keys()
        .iter()
        .map(|key| (key.key, key.state))
        .collect();
    assert_eq!(
        keys[..2],
        [(30, 1), (30, 0)],
        "the copy and its release, past the grab: {keys:?}"
    );
    // The real key's release goes through the keyboard once the grab is gone: a release of a
    // key the document already heard released, which a client ignores.
    assert_eq!(
        keys[2..],
        [(30, 0), (30, 1), (30, 0)],
        "the next key reaches the document through the keyboard: {keys:?}"
    );
}

/// An on-screen keyboard's own key, with nothing real held, is not a copy: it goes to the
/// method's grab like any key, and the document hears nothing until the method sends it back.
#[test]
fn an_on_screen_keyboards_own_key_goes_to_the_grab() {
    let mut typing = Typing::start();
    typing.enable_and_grab();
    let board =
        VirtualBoard::start(typing.fixture.client(typing.client).globals()).expect("the board");
    typing.pump();
    board.keymap(KEYMAP).expect("keymap");
    typing.pump();
    board.key(0, 30, 1).expect("press");
    typing.pump();
    board.key(0, 30, 0).expect("release");
    typing.pump();
    let grabbed: Vec<_> = typing
        .ime
        .method_events()
        .iter()
        .filter_map(|line| match line.event {
            InputMethodEvent::GrabKey { key, state } => Some((key, state)),
            _ => None,
        })
        .collect();
    assert_eq!(grabbed, vec![(30, 1), (30, 0)], "the method had the key");
    assert!(
        typing.ime.document_keys().is_empty(),
        "the document heard nothing: {:?}",
        typing.ime.document_keys()
    );
}

/// A method that never grabs does not stall typing: after the hold limit the key goes to the
/// document.
#[test]
fn a_method_that_never_grabs_lets_the_key_through_after_the_hold_limit() {
    let mut typing = Typing::start();
    typing.ime.enable().expect("enable");
    typing.pump();
    let keyboard = typing.keyboard;
    typing
        .fixture
        .input()
        .key(keyboard, KeyCode(30), Edge::Press);
    typing.pump();
    assert!(typing.ime.document_keys().is_empty(), "the key waits");
    typing
        .fixture
        .advance(casement_wayland::text_input::HOLD_LIMIT);
    typing.pump();
    assert_eq!(
        typing
            .ime
            .document_keys()
            .iter()
            .map(|key| (key.key, key.state))
            .collect::<Vec<_>>(),
        vec![(30, 1)],
        "the hold timer lets the key go to the document"
    );
}

/// Preedit then commit_string: the transcript matches tests/golden/transcripts/text_input-commit.txt.
#[test]
fn ime_commit_transcript() {
    let mut typing = Typing::start();
    typing.enable_and_grab();
    typing.ime.set_preedit("ni", 2, 2).expect("preedit");
    typing.ime.commit_batch().expect("commit");
    typing.pump();
    typing.ime.commit_string("你").expect("commit_string");
    typing.ime.commit_batch().expect("commit");
    typing.pump();
    let lines = typing.ime.field_lines();
    check_golden(&golden("text_input-commit.txt"), &lines).expect("the transcript");
    let _ = typing.client;
}

/// The IME starts after the app (fcitx5 usually does). The field got `enter` with the focus
/// although no input method existed, enabled itself with its state, and the input method that
/// binds later is activated for it with that state, then types into it.
#[test]
fn an_input_method_that_starts_after_the_field_is_enabled_gets_the_field() {
    let mut typing = Typing::start_with(false);
    assert!(
        typing
            .ime
            .field_events()
            .iter()
            .any(|line| line.event == TextInputEvent::Enter),
        "the field got enter with no input method running"
    );
    typing
        .ime
        .enable_with_state("héllo", 3)
        .expect("enable with state");
    typing.pump();
    typing.start_method();
    let seen: Vec<_> = typing
        .ime
        .method_events()
        .iter()
        .map(|line| line.event.clone())
        .collect();
    assert_eq!(
        seen,
        [
            InputMethodEvent::Activate,
            InputMethodEvent::SurroundingText {
                text: "héllo".to_owned(),
                cursor: 3,
                anchor: 3
            },
            InputMethodEvent::ContentType {
                hint: 0,
                purpose: 13
            },
            InputMethodEvent::Done,
        ],
        "activate, the field's state, done"
    );
    typing.ime.grab_keyboard().expect("grab");
    typing.pump();
    typing.ime.commit_string("你").expect("commit_string");
    typing.ime.commit_batch().expect("commit");
    typing.pump();
    assert!(
        typing
            .ime
            .field_events()
            .iter()
            .any(|line| line.event == TextInputEvent::CommitString(Some("你".to_owned()))),
        "the late input method types into the field"
    );
}

/// The input method quits and another starts: the field stays enabled (no `leave`, the
/// protocol has no event for a missing method) and the next method is activated for it.
#[test]
fn a_field_stays_enabled_when_its_input_method_goes_away() {
    let mut typing = Typing::start();
    typing.enable_and_grab();
    typing.ime.destroy_method().expect("destroy the method");
    typing.pump();
    assert!(
        typing
            .ime
            .field_events()
            .iter()
            .all(|line| line.event != TextInputEvent::Leave),
        "the field was not left"
    );
    let activations = |typing: &Typing| {
        typing
            .ime
            .method_events()
            .iter()
            .filter(|line| line.event == InputMethodEvent::Activate)
            .count()
    };
    assert_eq!(activations(&typing), 1);
    typing.start_method();
    assert_eq!(activations(&typing), 2, "the next method is activated");
}
