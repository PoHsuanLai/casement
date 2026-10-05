//! `text_input` (P0-H7).

use crate::xdg_shell::{boot, check_golden, click, golden, map_toplevel, pointer, settle, windows};
use casement_core::DeviceId;
use casement_core::input::{DeviceClass, Edge, KeyCode};
use casement_core::policy::ConnectionRole;
use casement_testclient::ime::input_method::InputMethodEvent;
use casement_testclient::ime::session::ImeSession;
use casement_testclient::ime::text_input::TextInputEvent;
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testkit::{ClientId, Fixture, FixtureSpec, TestOrigin};

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
        let (mut fixture, client) = boot();
        let base = WmBase::bind(fixture.client(client), 7);
        let window = map_toplevel(&mut fixture, client, &base, (400, 300));
        let ime = ImeSession::start(fixture.client(client).globals()).expect("the session");
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

/// Every connection sees the text-input manager (I7 shows it to everyone).
#[test]
fn text_input_is_advertised_to_every_role() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let public = fixture.connect(ConnectionRole::Public);
    let advertised = fixture
        .client(public)
        .globals()
        .contents()
        .with_list(|list| {
            list.iter().any(|global| {
                global.interface == "zwp_text_input_manager_v3" && global.version == 1
            })
        });
    assert!(advertised, "zwp_text_input_manager_v3 v1 is advertised");
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
