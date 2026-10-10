//! `wlr_toplevel` (P1-M2): the wlr window list. Ignored until the lane advertises
//! `zwlr_foreign_toplevel_manager_v1` and writes `casement_testclient::manage::wlr_toplevel`.

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};

/// A mapped window is listed with its title, app id, state and output; closing it sends
/// `closed`.
#[test]
#[ignore = "P1-M2"]
fn a_window_is_listed_with_its_state_and_withdrawn_when_it_closes() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-M2: map a window, read the handle, close it");
}

/// activate, maximize, minimize and close on a handle reach the same transitions the window's
/// own requests take.
#[test]
#[ignore = "P1-M2"]
fn handle_requests_move_the_window_like_its_own_requests() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-M2: each request, compare the model");
}

/// `set_rectangle` is stored for the minimise animation.
#[test]
#[ignore = "P1-M2"]
fn the_minimise_rectangle_is_stored() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    todo!("P1-M2: set_rectangle, minimise, read the target");
}
