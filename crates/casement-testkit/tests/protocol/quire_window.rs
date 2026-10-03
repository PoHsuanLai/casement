//! `quire_window` (P0-H2).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Quire_window_manager_v1 version 1 is advertised to public clients (C50: shell-host's probe).
#[test]
#[ignore = "P0-H2"]
fn the_global_is_advertised_from_p0() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H2: quire_window_manager_v1 version 1 is advertised to public clients (C50: shell-host's probe)"
    )
}

/// The key window receives emphasis(key), the others inactive.
#[test]
#[ignore = "P0-H2"]
fn emphasis_follows_focus() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H2: the key window receives emphasis(key), the others inactive")
}

/// A second get_window on one toplevel raises already_constructed.
#[test]
#[ignore = "P0-H2"]
fn a_second_get_window_is_already_constructed() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H2: a second get_window on one toplevel raises already_constructed")
}

/// Request_placement with a press serial is stored in the model, not acted on at P0.
#[test]
#[ignore = "P0-H2"]
fn request_placement_is_stored() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H2: request_placement with a press serial is stored in the model, not acted on at P0")
}
