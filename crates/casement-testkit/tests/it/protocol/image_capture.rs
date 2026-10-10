//! `image_capture` (P1-C1): output capture through ext-image-copy-capture. Ignored until the
//! lane advertises the two managers and writes `casement_testclient::capture`.

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// The captured frame equals the frame scanout shows, byte for byte on pixman.
#[test]
#[ignore = "P1-C1"]
fn a_capture_equals_the_scanout_frame() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-C1: capture an output, compare with fixture.shot(Output)");
}

/// The session announces the buffer constraints; a buffer that does not match is `failed`.
#[test]
#[ignore = "P1-C1"]
fn a_mismatched_buffer_fails_the_frame() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-C1: wrong size buffer");
}

/// An unplugged output stops its sessions.
#[test]
#[ignore = "P1-C1"]
fn an_unplugged_output_stops_its_sessions() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-C1: unplug while a session runs");
}
