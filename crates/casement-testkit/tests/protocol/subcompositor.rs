//! `subcompositor` (P0-H1).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Place_above/place_below order and synchronised commits reach the frame together.
#[test]
#[ignore = "P0-H1"]
fn subsurfaces_stack_and_sync() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H1: place_above/place_below order and synchronised commits reach the frame together")
}

/// Making a surface its own subsurface is the bad_surface error and disconnects only that client.
#[test]
#[ignore = "P0-H1"]
fn a_subsurface_of_itself_is_a_protocol_error() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H1: making a surface its own subsurface is the bad_surface error and disconnects only that client"
    )
}
