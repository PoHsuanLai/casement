//! A window casement frames is clipped at its two bottom corners (it needs the GLES renderer,
//! here llvmpipe: pixman draws the corners square). The window is one solid colour that
//! nothing else on the desktop has, so its extent can be read back from the shot.

use crate::scene::desk::{Desk, Dpi};
use casement_backend::SoftRenderer;
use casement_testclient::misc::decoration::DecorationManager;
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::{Fixture, FixtureSpec};

/// The app the window belongs to.
const APP: &str = "org.example.clip";

/// The window's one colour, ARGB.
const CLIENT: u32 = 0xff2a_3fd0;

/// The same colour as the shot's R, G, B, A bytes.
const CLIENT_RGBA: [u8; 4] = [0x2a, 0x3f, 0xd0, 0xff];

/// The window radius in pixels (1x).
const RADIUS: u32 = 10;

#[test]
fn a_framed_windows_bottom_corners_are_round_and_the_top_ones_square() {
    let spec = FixtureSpec::desktop().with_renderer(SoftRenderer::Llvmpipe);
    let mut fixture = Fixture::start(spec).expect("fixture");
    let mut desk = Desk::at(&mut fixture);
    let manager = DecorationManager::bind(desk.fixture.client(desk.id)).expect("advertised");
    let surface = desk
        .fixture
        .client(desk.id)
        .create_surface()
        .expect("a surface");
    let top = desk.base.toplevel_on(desk.fixture.client(desk.id), surface);
    let decoration = manager.decorate(desk.fixture.client(desk.id), &top);
    top.set_app_id(APP);
    top.commit_bare();
    desk.fixture.pump();
    assert!(top.ack_latest().is_some(), "the initial configure arrives");
    let buffer = shm_buffer(
        desk.fixture.client(desk.id),
        Dpi::One.buffer((400, 300)),
        Pattern::Solid(CLIENT),
    );
    top.show(&buffer);
    desk.fixture.pump();
    desk.kept.keep((buffer, decoration, manager, top));
    let shot = desk.shoot();

    let (left, top_row, right, bottom) = extent(&shot).expect("the window is on screen");
    assert_eq!(
        (right - left + 1, bottom - top_row + 1),
        (400, 300),
        "the content is whole at its sides and its top"
    );
    let at = |x: u32, y: u32| shot.get_pixel(x, y).0;
    let inset = RADIUS + 2;
    for (name, x, y, want) in [
        ("bottom left corner", left, bottom, false),
        ("bottom right corner", right, bottom, false),
        (
            "12 px into the bottom left",
            left + inset,
            bottom - inset,
            true,
        ),
        (
            "12 px into the bottom right",
            right - inset,
            bottom - inset,
            true,
        ),
        ("top left, square", left, top_row, true),
        ("top right, square", right, top_row, true),
        ("left edge above the curve", left, bottom - inset, true),
        ("bottom edge beside the curve", left + inset, bottom, true),
    ] {
        assert_eq!(at(x, y) == CLIENT_RGBA, want, "{name} at ({x}, {y})");
    }
}

/// The left, top, right and bottom pixel of the window's colour in `shot`.
fn extent(shot: &image::RgbaImage) -> Option<(u32, u32, u32, u32)> {
    shot.enumerate_pixels()
        .filter(|(_, _, pixel)| pixel.0 == CLIENT_RGBA)
        .fold(None, |found, (x, y, _)| match found {
            None => Some((x, y, x, y)),
            Some((l, t, r, b)) => Some((l.min(x), t.min(y), r.max(x), b.max(y))),
        })
}
