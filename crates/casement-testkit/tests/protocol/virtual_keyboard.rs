//! `virtual_keyboard` (P0-H7).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

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
#[ignore = "P0-H7: written; runs once P0-B-headless (Fixture::start) and P0-T-client (TestClient) land"]
fn the_virtual_keyboard_manager_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    assert!(sees_virtual_keyboard(&mut fixture, ConnectionRole::Public));
    let sandboxed = ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    });
    assert!(!sees_virtual_keyboard(&mut fixture, sandboxed));
}

/// A virtual keyboard's key enters the model as Origin::Virtual(Keyboard) and resets idleness (I8).
///
/// Steps: a client creates a keyboard with `ime::virtual_keyboard::{bind_virtual_keyboard,
/// send_keymap}`, a second client holds an idle notification of 30 s (H8's helper) and the
/// first sends `key` at 20 s: `idled` arrives at 50 s, not 30 s; the model saw the key as
/// `Origin::Virtual(Keyboard)` (the fixture's input log). A `key` before the keymap is the
/// protocol error `no_keymap`.
#[test]
#[ignore = "P0-H7: written; runs once P0-B-headless (Fixture::start) and P0-T-client (TestClient) land"]
fn virtual_keys_are_origin_virtual_and_count_as_activity() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-T-client: the steps in this test's doc (ime::virtual_keyboard on the client's queue)")
}
