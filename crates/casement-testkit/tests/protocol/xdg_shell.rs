//! `xdg_shell` (P0-H2).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// The first window is centred in the usable area, the next one cascade step further (floating only).
#[test]
#[ignore = "P0-H2"]
fn new_windows_centre_then_cascade() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H2: the first window is centred in the usable area, the next one cascade step further (floating only)"
    )
}

/// Move/resize with a serial that answers no press does nothing; with a button serial they start a grab.
#[test]
#[ignore = "P0-H2"]
fn move_and_resize_need_a_press_serial() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H2: move/resize with a serial that answers no press does nothing; with a button serial they start a grab"
    )
}

/// A press on an inactive window activates it and sends activated in a configure.
#[test]
#[ignore = "P0-H2"]
fn click_to_focus() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H2: a press on an inactive window activates it and sends activated in a configure")
}

/// P0 never sends tiled_* states.
#[test]
#[ignore = "P0-H2"]
fn a_floating_configure_has_no_tiled_edges() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H2: P0 never sends tiled_* states")
}

/// Create, configure, ack, map, close: the transcript matches tests/golden/transcripts/xdg_shell-map.txt.
#[test]
#[ignore = "P0-H2"]
fn toplevel_transcript_golden() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!(
        "P0-H2: create, configure, ack, map, close: the transcript matches tests/golden/transcripts/xdg_shell-map.txt"
    )
}
