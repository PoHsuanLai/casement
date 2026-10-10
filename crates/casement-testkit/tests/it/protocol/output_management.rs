//! `output_management` (P1-M1): wlr-randr-style transcripts against virtual outputs. Ignored
//! until the lane advertises `zwlr_output_manager_v1` and writes
//! `casement_testclient::manage::output_management`.

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// Binding the manager lists one head per output with its modes, then `done`.
#[test]
#[ignore = "P1-M1"]
fn the_manager_lists_every_output_as_a_head() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-M1: bind, read the heads, compare with the model's outputs");
}

/// An applied configuration that changes the mode reaches `ApplyOutput`, and the head says so.
#[test]
#[ignore = "P1-M1"]
fn an_applied_configuration_changes_the_mode() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-M1: apply a new mode, expect `succeeded` and a new head state");
}

/// A test changes nothing; a configuration built at an old serial is `cancelled`; a mode the
/// display does not offer is `failed`.
#[test]
#[ignore = "P1-M1"]
fn test_stale_and_refused_configurations_answer_correctly() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-M1: test, stale serial, unsupported mode");
}
