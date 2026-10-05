//! `tablet` (P0-H4).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Zwp_tablet_manager_v2 gives a tablet seat for wl_seat.
#[test]
#[ignore = "body is todo!(): the P0-H4 stub was never written (the H4 patch has no tablet step)"]
fn the_tablet_seat_is_advertised() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H4: zwp_tablet_manager_v2 gives a tablet seat for wl_seat")
}
