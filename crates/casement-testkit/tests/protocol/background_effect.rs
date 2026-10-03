//! `background_effect` (P0-H8).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Set_blur_region is stored for the surface and cleared by an empty region (H8 exit).
#[test]
#[ignore = "P0-H8"]
fn a_blur_region_is_stored() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H8: set_blur_region is stored for the surface and cleared by an empty region (H8 exit)"
    )
}
