//! `tablet` (P0-H4).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Zwp_tablet_manager_v2 gives a tablet seat for wl_seat.
#[test]
#[ignore = "P0-H4"]
fn the_tablet_seat_is_advertised() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H4: zwp_tablet_manager_v2 gives a tablet seat for wl_seat")
}
