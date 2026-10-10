//! `gamma` (P1-M1): the colour curve of an output. Ignored until the lane advertises
//! `zwlr_gamma_control_manager_v1`.

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// The control announces the table size; a ramp of that size is accepted and reaches the
/// backend as `Effect::SetGamma`.
#[test]
#[ignore = "P1-M1"]
fn a_ramp_of_the_announced_size_reaches_the_backend() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-M1: gamma_size, set_gamma, read the ramp back from the backend");
}

/// A ramp of the wrong size is `invalid_gamma`; destroying the control restores the identity
/// curve.
#[test]
#[ignore = "P1-M1"]
fn a_wrong_size_is_an_error_and_destroying_restores_identity() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-M1: wrong size, then destroy");
}
