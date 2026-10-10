//! X11 clients through xwayland-satellite (P1-X1), with the fixture's `FakeSpawner`. The real
//! satellite runs in `dev/accept/xwayland-*.sh`.

use casement::xwayland::{Xwayland, XwaylandConfig};
use casement_testkit::fakes::FakeSpawner;

/// Satellite is started with the listening sockets and the configured binary.
#[test]
#[ignore = "P1-X1"]
fn satellite_starts_with_its_listen_sockets() {
    let mut spawner = FakeSpawner::new();
    let config = XwaylandConfig::satellite("/opt/satellite");
    let _running = Xwayland::start(&config, &mut spawner);
    todo!("P1-X1: one spawn, argv[0] is the configured path, the sockets are passed");
}

/// Off starts nothing and exports no `DISPLAY`.
#[test]
#[ignore = "P1-X1"]
fn off_starts_nothing() {
    let spawner = FakeSpawner::new();
    assert!(spawner.started().is_empty());
    todo!("P1-X1: Xwayland::start with XwaylandConfig::off() makes no spawn");
}

/// An X11 window is a Wayland window whose app id is its `WM_CLASS`; its facts (override-redirect,
/// scale on two outputs, activation) are the suite's rows.
#[test]
#[ignore = "P1-X1"]
fn an_x11_window_takes_its_wm_class_as_its_app_id() {
    todo!("P1-X1: the XWayland suite against casement-x11probe");
}
