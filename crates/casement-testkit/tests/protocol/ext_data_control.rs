//! `ext_data_control` (P0-H5).

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

/// A Sandboxed connection does not see ext_data_control_manager_v1 (I7, H5 exit).
#[test]
#[ignore = "P0-H5: needs the fixture (P0-B-headless, P0-B-loop) and the test client (P0-T-client) merged"]
fn data_control_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let sandboxed = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    let public = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    assert!(
        !advertises(fixture.client(sandboxed), "ext_data_control_manager_v1"),
        "a sandboxed client must not see ext_data_control_manager_v1"
    );
    assert!(
        advertises(fixture.client(public), "ext_data_control_manager_v1"),
        "a public client sees ext_data_control_manager_v1"
    );
}

/// A clipboard manager is told of every new selection.
#[test]
#[ignore = "P0-H5"]
fn a_manager_sees_selection_changes() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H5: a clipboard manager is told of every new selection")
}
