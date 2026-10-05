//! `commit_timing` (P0-H1).

use casement_core::geometry::PhysicalSize;
use casement_testclient::misc::commit_timing::{CommitTiming, set_timestamp};
use casement_testclient::surface::subsurface::Compositor;
use casement_testclient::{Pattern, shm_buffer};
use std::time::Duration;

use casement_core::policy::ConnectionRole;
use casement_testkit::{ClientId, Fixture, FixtureSpec};

fn boot() -> (Fixture, ClientId) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    (fixture, id)
}

/// The (interface, code) of the protocol error the compositor killed `id` with.
fn fault(fixture: &mut Fixture, id: ClientId) -> (String, u32) {
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("the compositor posted an error");
    (error.interface, error.code)
}

const SIZE: PhysicalSize = PhysicalSize { w: 16, h: 16 };

/// A commit timed 50 ms ahead appears at the first vblank at or after it, on virtual time.
///
/// Observed through the buffers: the timed commit replaces the first buffer, which the
/// compositor then releases, so its release says the timed commit was applied.
#[test]
fn a_timestamped_commit_waits_for_its_instant() {
    let (mut fixture, id) = boot();
    let started = fixture.model().now();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let timer = CommitTiming::bind(client).for_surface(client, &surface);
    let first = shm_buffer(client, SIZE, Pattern::Solid(0xffff_0000));
    let second = shm_buffer(client, SIZE, Pattern::Solid(0xff00_00ff));
    surface.attach(&first);
    surface.commit();
    surface.attach(&second);
    set_timestamp(&timer, started.0 + Duration::from_millis(50));
    surface.commit();
    fixture.pump();
    assert!(!first.is_released());
    // The vblanks before the instant (every 16.7 ms) leave the commit held.
    fixture.advance(Duration::from_millis(49));
    assert!(!first.is_released(), "49 ms in");
    // The first vblank at or after 50 ms is within 16.7 ms of it.
    fixture.advance(Duration::from_millis(20));
    assert!(first.is_released(), "69 ms in");
    assert!(fixture.client(id).protocol_error().is_none());
}

/// A second timer on one surface is wp_commit_timing_manager_v1.commit_timer_exists (0), posted
/// on the surface.
#[test]
fn a_second_timer_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let manager = CommitTiming::bind(client);
    let _first = manager.for_surface(client, &surface);
    let _second = manager.for_surface(client, &surface);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wl_surface".to_owned(), 0));
}

/// Two timestamps in one commit are wp_commit_timer_v1.timestamp_exists (1).
#[test]
fn two_timestamps_in_one_commit_are_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let timer = CommitTiming::bind(client).for_surface(client, &surface);
    set_timestamp(&timer, Duration::from_secs(100));
    set_timestamp(&timer, Duration::from_secs(101));
    fixture.pump();
    assert_eq!(
        fault(&mut fixture, id),
        ("wp_commit_timer_v1".to_owned(), 1)
    );
}
