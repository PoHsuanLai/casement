//! `foreign_toplevel_list` (P0-H8).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

/// Each mapped window appears with title and app id; closing it sends closed.
#[test]
#[ignore = "P0-H8"]
fn windows_are_listed_and_closed() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H8: each mapped window appears with title and app id; closing it sends closed")
}

/// A Sandboxed connection does not see ext_foreign_toplevel_list_v1 (I7).
#[test]
#[ignore = "P0-H8"]
fn the_list_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    let _ = fixture.client(client);
    todo!("P0-H8: a Sandboxed connection does not see ext_foreign_toplevel_list_v1 (I7)")
}
