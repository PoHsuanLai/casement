//! `alpha_modifier` (P0-H1).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A surface at half alpha composites at half (pixman golden).
#[test]
#[ignore = "P0-H1"]
fn the_multiplier_reaches_the_frame() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H1: a surface at half alpha composites at half (pixman golden)")
}
