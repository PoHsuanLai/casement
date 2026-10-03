//! `fractional_scale` (P0-H1).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// At scale 1.5 the surface receives preferred_scale(180) before its first xdg configure (shell-host FINDINGS 167).
#[test]
#[ignore = "P0-H1"]
fn preferred_scale_comes_before_the_first_configure() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H1: at scale 1.5 the surface receives preferred_scale(180) before its first xdg configure (shell-host FINDINGS 167)"
    )
}
