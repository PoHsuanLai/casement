//! `layer_shell` (P0-H3).

use casement_core::policy::ConnectionRole;
use casement_testclient::TestClient;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

/// Whether the registry the client saw lists `interface`.
fn advertises(client: &mut TestClient, interface: &str) -> bool {
    client
        .globals()
        .contents()
        .clone_list()
        .iter()
        .any(|global| global.interface == interface)
}

/// An Exclusive layer gets wl_keyboard.enter with the current modifiers on the commit that maps it (A16).
#[test]
#[ignore = "P0-H3"]
fn an_exclusive_layer_takes_the_keyboard_on_its_mapping_commit() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H3: an Exclusive layer gets wl_keyboard.enter with the current modifiers on the commit that maps it (A16)"
    )
}

/// A layer mapping under a pointer that does not move still receives enter (SP1 A1).
#[test]
#[ignore = "P0-H3"]
fn a_layer_under_a_resting_pointer_gets_enter() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H3: a layer mapping under a pointer that does not move still receives enter (SP1 A1)")
}

/// A bar's exclusive zone is subtracted from a maximised window's configure (A16).
#[test]
#[ignore = "P0-H3"]
fn exclusive_zones_shrink_maximised_windows() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H3: a bar's exclusive zone is subtracted from a maximised window's configure (A16)")
}

/// A Sandboxed connection does not see zwlr_layer_shell_v1 (I7).
#[test]
#[ignore = "P0-H3: needs the fixture (P0-B-headless, P0-B-loop) and the test client (P0-T-client) merged"]
fn layer_shell_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let sandboxed = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    let public = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    assert!(
        !advertises(fixture.client(sandboxed), "zwlr_layer_shell_v1"),
        "a sandboxed client must not see zwlr_layer_shell_v1"
    );
    assert!(
        advertises(fixture.client(public), "zwlr_layer_shell_v1"),
        "a public client sees zwlr_layer_shell_v1"
    );
}
