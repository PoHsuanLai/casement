//! `output_power` (P1-M1): output power works on virtual outputs. Ignored until the lane
//! advertises `zwlr_output_power_manager_v1`.

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Turning an output off darkens it (no more frames presented) and the control reports `off`;
/// turning it on lights it again.
#[test]
#[ignore = "P1-M1"]
fn power_off_darkens_a_virtual_output_and_on_lights_it() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-M1: set_mode off, frames stop; set_mode on, frames resume");
}

/// A second control on the same output is `failed`.
#[test]
#[ignore = "P1-M1"]
fn a_second_control_on_an_output_fails() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-M1: two get_output_power on one output");
}
