//! `wlr_data_control` (P0-H5).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

/// A Sandboxed connection does not see zwlr_data_control_manager_v1 (I7).
#[test]
#[ignore = "P0-H5"]
fn data_control_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    let _ = fixture.client(client);
    todo!("P0-H5: a Sandboxed connection does not see zwlr_data_control_manager_v1 (I7)")
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
