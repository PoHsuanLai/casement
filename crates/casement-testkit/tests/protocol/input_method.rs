//! `input_method` (P0-H7).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

/// The input method's commit_string reaches the focused text field.
#[test]
#[ignore = "P0-H7"]
fn an_input_method_commits_to_the_focused_field() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H7: the input method's commit_string reaches the focused text field")
}

/// A Sandboxed connection does not see zwp_input_method_manager_v2 (I7).
#[test]
#[ignore = "P0-H7"]
fn input_method_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    let _ = fixture.client(client);
    todo!("P0-H7: a Sandboxed connection does not see zwp_input_method_manager_v2 (I7)")
}
