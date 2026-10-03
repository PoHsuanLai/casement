//! `xdg_activation` (P0-H2).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A token from a recent press of the focused app activates; a stale or foreign one only marks attention.
#[test]
#[ignore = "P0-H2"]
fn activation_follows_the_a7_rule() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H2: a token from a recent press of the focused app activates; a stale or foreign one only marks attention"
    )
}
