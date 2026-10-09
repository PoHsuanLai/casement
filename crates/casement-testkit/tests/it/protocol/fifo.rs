//! `fifo` (P0-H1).

use casement_core::OutputName;
use casement_core::geometry::PhysicalSize;
use casement_testclient::misc::fifo::Fifo;
use casement_testclient::surface::subsurface::Compositor;
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::FrameCount;

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

/// A second fifo commit is applied only after the first was presented.
///
/// Observed through the buffers: committing the second buffer replaces the first, which the
/// compositor then releases, so the first buffer's release says the second commit was applied.
#[test]
fn a_fifo_barrier_waits_for_the_next_vblank() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let fifo = Fifo::bind(client).for_surface(client, &surface);
    let first = shm_buffer(client, SIZE, Pattern::Solid(0xffff_0000));
    let second = shm_buffer(client, SIZE, Pattern::Solid(0xff00_00ff));
    surface.attach(&first);
    fifo.set_barrier();
    surface.commit();
    surface.attach(&second);
    fifo.wait_barrier();
    surface.commit();
    fixture.pump();
    assert!(!first.is_released(), "held until a frame");
    fixture.frames(
        &OutputName::parse("HEADLESS-1").expect("a name"),
        FrameCount(1),
    );
    fixture.pump();
    assert!(first.is_released(), "applied after the frame");
    assert!(fixture.client(id).protocol_error().is_none());
}

/// A second fifo object on one surface is wp_fifo_manager_v1.already_exists (0), posted on the
/// surface.
#[test]
fn a_second_fifo_object_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let manager = Fifo::bind(client);
    let _first = manager.for_surface(client, &surface);
    let _second = manager.for_surface(client, &surface);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wl_surface".to_owned(), 0));
}

/// Requests on a fifo object whose surface is gone are wp_fifo_v1.surface_destroyed (0).
#[test]
fn a_fifo_outliving_its_surface_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let fifo = Fifo::bind(client).for_surface(client, &surface);
    surface.destroy();
    fifo.set_barrier();
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wp_fifo_v1".to_owned(), 0));
}
