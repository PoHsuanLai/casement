//! `tablet` (P0-H4).

use casement_core::policy::ConnectionRole;
use casement_testclient::TestClient;
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::seat::tablet::TabletSeat;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

/// The version of `interface` the registry the client saw lists, if it lists it.
fn version_of(client: &mut TestClient, interface: &str) -> Option<u32> {
    client
        .globals()
        .contents()
        .clone_list()
        .iter()
        .find(|global| global.interface == interface)
        .map(|global| global.version)
}

/// Zwp_tablet_manager_v2 gives a tablet seat for wl_seat.
///
/// The global is advertised at version 1 to every connection (the I7 table: a tablet is input
/// for the client's own windows, nothing a sandbox must not see). The tablet seat of a
/// `wl_seat` is made without a protocol error, and since no tablet is attached to the
/// headless seat it announces no tablet, tool or pad.
#[test]
fn the_tablet_seat_is_advertised() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let public = fixture.connect(ConnectionRole::Public);
    let sandboxed = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    fixture.pump();
    assert_eq!(
        version_of(fixture.client(public), "zwp_tablet_manager_v2"),
        Some(1)
    );
    assert_eq!(
        version_of(fixture.client(sandboxed), "zwp_tablet_manager_v2"),
        Some(1),
        "a sandboxed client sees the tablet manager too"
    );
    for id in [public, sandboxed] {
        let pointer = Pointer::bind(fixture.client(id)).expect("a seat");
        let tablets = TabletSeat::bind(fixture.client(id), pointer.seat()).expect("a tablet seat");
        let done = fixture.client(id).sync().expect("a sync");
        fixture.pump();
        assert!(done.is_done(), "the compositor answered after the request");
        assert!(
            fixture.client(id).protocol_error().is_none(),
            "a tablet seat for a wl_seat is not an error"
        );
        let announced: Vec<String> = fixture
            .client(id)
            .transcript()
            .0
            .iter()
            .filter(|line| line.object.starts_with("zwp_tablet_seat_v2#"))
            .map(|line| line.event.clone())
            .collect();
        assert!(announced.is_empty(), "no tablet is attached: {announced:?}");
        let _ = tablets;
    }
}
