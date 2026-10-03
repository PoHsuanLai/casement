//! `data_device` (P0-H5).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A selection set by A is offered to B when B gains keyboard focus.
#[test]
#[ignore = "P0-H5"]
fn a_selection_reaches_the_focused_client() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H5: a selection set by A is offered to B when B gains keyboard focus")
}

/// A drag from a window drops on a layer surface (H5 exit).
#[test]
#[ignore = "P0-H5"]
fn drag_and_drop_into_a_layer_surface() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H5: a drag from a window drops on a layer surface (H5 exit)")
}
