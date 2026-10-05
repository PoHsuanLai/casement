//! The client infrastructure against the real compositor (P0-T-client).

use casement_core::geometry::PhysicalSize;
use casement_core::policy::ConnectionRole;
use casement_testclient::{Pattern, Pumped, shm_buffer};
use casement_testkit::{Fixture, FixtureSpec};

#[test]
fn a_client_over_a_socket_pair_sees_globals_and_goes_quiet() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    assert!(
        !fixture
            .client(id)
            .globals()
            .contents()
            .clone_list()
            .is_empty()
    );
    assert_eq!(fixture.pump(), Pumped::Quiet);
    assert_eq!(fixture.client(id).pump(), Pumped::Quiet);
}

#[test]
fn an_shm_buffer_is_made_without_a_protocol_error() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    let _buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 64, h: 32 },
        Pattern::CoordinateEncoding,
    );
    // The requests are served (that is a pump that moved), and nothing is left over.
    assert_eq!(fixture.pump(), Pumped::Moved);
    assert!(fixture.client(id).protocol_error().is_none());
    assert_eq!(fixture.pump(), Pumped::Quiet);
}

/// casement-probe's exit code for the arguments `args`, run with nothing from this environment
/// (no compositor to connect to: a usage error must be found before it tries).
fn probe_exit(args: &[&str]) -> Option<i32> {
    std::process::Command::new(env!("CARGO_BIN_EXE_casement-probe"))
        .env_clear()
        .args(args)
        .output()
        .expect("casement-probe runs")
        .status
        .code()
}

#[test]
fn the_probe_rejects_unknown_flags_with_the_usage_code() {
    assert_eq!(probe_exit(&["toplevel", "--maximized"]), Some(64));
    assert_eq!(probe_exit(&["layer", "--maximised"]), Some(64));
    assert_eq!(probe_exit(&["toplevel", "--title"]), Some(64));
    assert_eq!(probe_exit(&["frobnicate"]), Some(64));
}
