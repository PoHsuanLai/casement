//! `layer_shell` (P0-H3).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

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
#[ignore = "P0-H3"]
fn layer_shell_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    let _ = fixture.client(client);
    todo!("P0-H3: a Sandboxed connection does not see zwlr_layer_shell_v1 (I7)")
}
