//! A window casement frames is clipped at its two bottom corners on every renderer: GLES (here
//! llvmpipe) with the shader for a texture and the piecewise draw for a single-pixel buffer, and
//! pixman by pieces. The window is one solid colour that nothing else on the desktop has, so its
//! extent can be read back from the shot.

use crate::scene::desk::{Desk, Dpi};
use casement_backend::SoftRenderer;
use casement_testclient::misc::decoration::DecorationManager;
use casement_testclient::misc::viewporter::{SinglePixel, Viewporter};
use casement_testclient::surface::subsurface::Surface;
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

/// How the window's buffer is made.
#[derive(Debug, Clone, Copy)]
enum Source {
    /// A shared-memory buffer of the window's size.
    Shm,
    /// A one-pixel buffer stretched to the window's size by a viewport.
    OnePixel,
}

#[test]
fn a_framed_windows_bottom_corners_are_round_on_gles() {
    round_and_square(SoftRenderer::Llvmpipe, Source::Shm);
}

#[test]
fn a_framed_windows_bottom_corners_are_round_on_pixman() {
    round_and_square(SoftRenderer::Pixman, Source::Shm);
}

#[test]
fn a_single_pixel_windows_bottom_corners_are_round_on_gles() {
    round_and_square(SoftRenderer::Llvmpipe, Source::OnePixel);
}

#[test]
fn a_single_pixel_windows_bottom_corners_are_round_on_pixman() {
    round_and_square(SoftRenderer::Pixman, Source::OnePixel);
}

/// A decorated 400x300 window drawn by `renderer` from `source`: its bottom corners are cut and
/// anti-aliased, its top corners and edges whole.
fn round_and_square(renderer: SoftRenderer, source: Source) {
    let shot = framed_shot(renderer, source);
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
        assert_eq!(
            is_client(at(x, y)),
            want,
            "{name} at ({x}, {y}) on {renderer:?}"
        );
    }
    // The curve is anti-aliased: a pixel the curve cuts through is neither the window's colour
    // nor as far from it as the corner pixel.
    let distance = |p: [u8; 4]| -> u32 {
        p.iter()
            .zip(CLIENT_RGBA)
            .map(|(a, b)| u32::from(a.abs_diff(b)))
            .sum()
    };
    for (name, x) in [("left", left + 6), ("right", right - 6)] {
        let cut = distance(at(x, bottom));
        assert!(
            cut > 0 && cut < distance(at(left, bottom)),
            "the curve is blended on the {name} on {renderer:?}: {cut}"
        );
    }
}

/// The shot of a decorated 400x300 window of `source` drawn by `renderer`.
fn framed_shot(renderer: SoftRenderer, source: Source) -> image::RgbaImage {
    let spec = FixtureSpec::desktop().with_renderer(renderer);
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
    match source {
        Source::Shm => {
            let buffer = shm_buffer(
                desk.fixture.client(desk.id),
                Dpi::One.buffer((400, 300)),
                Pattern::Solid(CLIENT),
            );
            top.show(&buffer);
            desk.kept.keep(buffer);
        }
        Source::OnePixel => {
            let client = desk.fixture.client(desk.id);
            let viewport = Viewporter::bind(client).viewport(
                client,
                &Surface {
                    surface: top.surface.clone(),
                },
            );
            viewport.destination(400, 300);
            let lift = |byte: u8| u32::from(byte) * 0x0101_0101;
            let [r, g, b, a] = CLIENT_RGBA.map(lift);
            let buffer = SinglePixel::bind(client).buffer(client, [r, g, b, a]);
            top.surface.attach(Some(&buffer), 0, 0);
            top.surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
            top.surface.commit();
            desk.kept.keep((buffer, viewport));
        }
    }
    desk.fixture.pump();
    desk.kept.keep((decoration, manager, top));
    desk.shoot()
}

/// Whether `pixel` is the window's colour (a one-pixel buffer may land a step off).
fn is_client(pixel: [u8; 4]) -> bool {
    pixel
        .iter()
        .zip(CLIENT_RGBA)
        .all(|(a, b)| a.abs_diff(b) <= 1)
}

/// The left, top, right and bottom pixel of the window's colour in `shot`.
fn extent(shot: &image::RgbaImage) -> Option<(u32, u32, u32, u32)> {
    shot.enumerate_pixels()
        .filter(|(_, _, pixel)| is_client(pixel.0))
        .fold(None, |found, (x, y, _)| match found {
            None => Some((x, y, x, y)),
            Some((l, t, r, b)) => Some((l.min(x), t.min(y), r.max(x), b.max(y))),
        })
}
