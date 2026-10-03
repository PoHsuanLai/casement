//! `single_pixel` (P0-H1).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A 1x1 buffer scaled by a viewport draws its colour over the destination.
#[test]
#[ignore = "P0-H1"]
fn a_single_pixel_buffer_draws_its_colour() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H1: a 1x1 buffer scaled by a viewport draws its colour over the destination")
}
