//! `single_pixel` (P0-H1).

use casement_testclient::misc::viewporter::{SinglePixel, Viewporter};
use casement_testclient::surface::subsurface::Compositor;

use casement_core::policy::ConnectionRole;
use casement_testkit::{ClientId, Fixture, FixtureSpec};

use crate::shoot;

fn boot() -> (Fixture, ClientId) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    (fixture, id)
}

/// A 1x1 buffer scaled by a viewport destination draws its colour over the destination.
#[test]
fn a_single_pixel_buffer_draws_its_colour() {
    let (mut fixture, id, base) = shoot::desk();
    let (_top, _kept) = shoot::map_with(&mut fixture, id, &base, |client, top| {
        let viewport = Viewporter::bind(client).viewport(client, &shoot::surface_of(top));
        viewport.destination(64, 32);
        let buffer = SinglePixel::bind(client).buffer(client, [u32::MAX, 0x8000_0000, 0, u32::MAX]);
        top.surface.attach(Some(&buffer), 0, 0);
        (buffer, viewport)
    });
    let window = shoot::windows(&fixture)[0];
    let shot = shoot::shot_of(&mut fixture, window);
    assert_eq!(shot.dimensions(), (64, 32), "the destination is the size");
    // Opaque orange: red full, green half, blue none.
    let wrong = shot
        .pixels()
        .filter(|pixel| {
            pixel.0[0] != 255
                || pixel.0[1].abs_diff(128) > 1
                || pixel.0[2] != 0
                || pixel.0[3] != 255
        })
        .count();
    assert_eq!(wrong, 0, "{wrong} pixels are not the buffer's colour");
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
