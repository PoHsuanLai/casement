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
