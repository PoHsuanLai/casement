//! `input_method` (P0-H7).

use crate::text_input::Typing;
use casement_core::policy::ConnectionRole;
use casement_testclient::ime::text_input::TextInputEvent;
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
fn input_method_is_advertised_to_public_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    assert!(sees_input_method(&mut fixture, ConnectionRole::Public));
}

/// The input method's commit_string reaches the focused text field: `commit_string` then `done`
/// carrying the serial of the field's own last `commit`.
#[test]
fn an_input_method_commits_to_the_focused_field() {
    let mut typing = Typing::start();
    typing.enable_and_grab();
    typing.ime.commit_string("你").expect("commit_string");
    typing.ime.commit_batch().expect("commit");
    typing.pump();
    let events: Vec<_> = typing
        .ime
        .field_events()
        .iter()
        .map(|line| line.event.clone())
        .collect();
    assert_eq!(
        events[events.len().saturating_sub(2)..],
        [
            TextInputEvent::CommitString(Some("你".to_owned())),
            TextInputEvent::Done(1)
        ]
    );
}

/// A Sandboxed connection does not see zwp_input_method_manager_v2 (I7).
#[test]
fn input_method_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let sandboxed = ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    });
    assert!(!sees_input_method(&mut fixture, sandboxed));
}
