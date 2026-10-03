//! `session_lock` (P0-H3).

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

/// While locked, keys and pointer reach only the lock surface.
#[test]
#[ignore = "P0-H3"]
fn lock_blocks_input_to_windows() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H3: while locked, keys and pointer reach only the lock surface")
}

/// Killing the locker keeps every output covered; nothing of the session is drawn.
#[test]
#[ignore = "P0-H3"]
fn a_dead_locker_leaves_the_lock_cover() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H3: killing the locker keeps every output covered; nothing of the session is drawn")
}

/// Locked arrives only after each output shows a lock surface or a cover.
#[test]
#[ignore = "P0-H3"]
fn locked_is_sent_once_every_output_is_covered() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H3: locked arrives only after each output shows a lock surface or a cover")
}

/// A Sandboxed connection does not see ext_session_lock_manager_v1 (I7).
#[test]
#[ignore = "P0-H3: needs the fixture (P0-B-headless, P0-B-loop) and the test client (P0-T-client) merged"]
fn session_lock_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let sandboxed = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    let public = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    assert!(
        !advertises(fixture.client(sandboxed), "ext_session_lock_manager_v1"),
        "a sandboxed client must not see ext_session_lock_manager_v1"
    );
    assert!(
        advertises(fixture.client(public), "ext_session_lock_manager_v1"),
        "a public client sees ext_session_lock_manager_v1"
    );
}
