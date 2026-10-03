//! `output` (P0-H1).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Three outputs at scales 1, 1.5 and 2 each advertise wl_output with name, mode and the integer scale ceiling; xdg-output gives logical size.
#[test]
#[ignore = "P0-H1"]
fn each_plugged_output_is_advertised_with_its_name_and_scale() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H1: three outputs at scales 1, 1.5 and 2 each advertise wl_output with name, mode and the integer scale ceiling; xdg-output gives logical size"
    )
}

/// Unplugging removes the global; a client bound to it sees it gone.
#[test]
#[ignore = "P0-H1"]
fn an_unplugged_output_global_is_withdrawn() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H1: unplugging removes the global; a client bound to it sees it gone")
}
