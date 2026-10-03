//! `input_method` (P0-H7).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

fn sees_input_method(fixture: &mut Fixture, role: ConnectionRole) -> bool {
    let client = fixture.connect(role);
    fixture
        .client(client)
        .globals()
        .contents()
        .with_list(|list| {
            list.iter().any(|global| {
                global.interface == "zwp_input_method_manager_v2" && global.version == 1
            })
        })
}

/// A Public connection sees zwp_input_method_manager_v2 (version 1).
#[test]
#[ignore = "P0-H7: written; runs once P0-B-headless (Fixture::start) and P0-T-client (TestClient) land"]
fn input_method_is_advertised_to_public_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    assert!(sees_input_method(&mut fixture, ConnectionRole::Public));
}

/// The input method's commit_string reaches the focused text field.
///
/// Steps: as in text_input's `ime_commit_transcript`; the field's transcript holds
/// `commit_string` then `done` carrying the serial of the field's own last `commit`.
#[test]
#[ignore = "P0-H7: written; runs once P0-B-headless (Fixture::start) and P0-T-client (TestClient) land"]
fn an_input_method_commits_to_the_focused_field() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-T-client: the steps in this test's doc (ime::input_method and ime::text_input on one queue)"
    )
}

/// A Sandboxed connection does not see zwp_input_method_manager_v2 (I7).
#[test]
#[ignore = "P0-H7: written; runs once P0-B-headless (Fixture::start) and P0-T-client (TestClient) land"]
fn input_method_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let sandboxed = ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    });
    assert!(!sees_input_method(&mut fixture, sandboxed));
}
