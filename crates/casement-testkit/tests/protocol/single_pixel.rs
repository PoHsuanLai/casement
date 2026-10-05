//! `single_pixel` (P0-H1).

use casement_testclient::misc::viewporter::{SinglePixel, Viewporter};
use casement_testclient::surface::subsurface::Compositor;

use casement_core::policy::ConnectionRole;
use casement_testkit::{ClientId, Fixture, FixtureSpec};

fn boot() -> (Fixture, ClientId) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    (fixture, id)
}

/// A 1x1 buffer scaled by a viewport draws its colour over the destination.
#[test]
#[ignore = "body is todo!(): needs a pixman golden of a mapped toplevel (P0-R-scene shoot(); P0-H2's toplevel helper is on master)"]
fn a_single_pixel_buffer_draws_its_colour() {
    todo!(
        "P0-H1: map a toplevel from a single-pixel buffer under a 64x32 viewport destination and read the pixels back"
    )
}

/// Every colour makes a buffer that attaches and commits, under a viewport destination, with no
/// protocol error.
#[test]
fn a_single_pixel_buffer_attaches_and_commits() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let viewport = Viewporter::bind(client).viewport(client, &surface);
    let pixels = SinglePixel::bind(client);
    for rgba in [
        [0, 0, 0, 0],
        [u32::MAX, 0, 0, u32::MAX],
        [0x8000_0000, 0x8000_0000, 0x8000_0000, 0x8000_0000],
    ] {
        let buffer = pixels.buffer(client, rgba);
        surface.attach_raw(&buffer);
        viewport.destination(64, 32);
        surface.commit();
    }
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
}
