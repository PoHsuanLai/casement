//! A window that draws its own frame is shaped by what its client said about it: `set_frame`
//! (compositor_shadow) clips its four corners and casts casement's shadow, saying nothing does
//! the same (without the shadow when the buffer holds a margin the toolkit drew its own in), and
//! `set_frame(client)` and full screen leave it as it is. The window is one solid colour that
//! nothing else on the desktop has, over a light wallpaper a shadow darkens.

use crate::scene::desk::{Desk, Dpi};
use casement_backend::SoftRenderer;
use casement_core::input::{ButtonCode, Edge};
use casement_testclient::Pattern;
use casement_testclient::quire::window::{FrameMode, WindowManager};
use casement_testkit::{Fixture, FixtureSpec};

/// The app the window belongs to.
const APP: &str = "org.example.round";

/// The window's one colour, ARGB.
const CLIENT: u32 = 0xff2a_3fd0;

/// The same colour as the shot's R, G, B, A bytes.
const CLIENT_RGBA: [u8; 4] = [0x2a, 0x3f, 0xd0, 0xff];

/// The wallpaper the shadow falls on: light, so that a black shadow darkens it. ARGB.
const WALLPAPER: u32 = 0xffe0_e0e0;

/// The radius the window asks for, in pixels (1x).
const RADIUS: u32 = 12;

/// The window's geometry, in pixels.
const SIZE: (u32, u32) = (400, 300);

/// How far a margin reaches round the geometry, when the buffer holds one.
const MARGIN: u32 = 20;

/// What the client says about its frame.
#[derive(Debug, Clone, Copy)]
enum Say {
    /// Nothing: a GTK or Electron app.
    Nothing,
    /// `set_frame(mode, RADIUS)`.
    Frame(FrameMode),
}

/// What the buffer holds.
#[derive(Debug, Clone, Copy)]
enum Buffer {
    /// The window geometry and nothing else.
    Whole,
    /// The geometry inset in a margin, as a toolkit that draws its shadow has it.
    Margin,
}

/// Where the window is.
#[derive(Debug, Clone, Copy)]
enum Placed {
    /// Floating on the desktop.
    Floating,
    /// Covering the output.
    FullScreen,
}

/// The desktop without the window, and with it.
struct Shots {
    bare: image::RgbaImage,
    shot: image::RgbaImage,
}

/// The left, top, right and bottom pixel of something in a shot, all inclusive.
type Rect = (u32, u32, u32, u32);

const RENDERERS: [SoftRenderer; 2] = [SoftRenderer::Pixman, SoftRenderer::Llvmpipe];

#[test]
fn a_window_that_asks_for_a_shadow_gets_round_corners_and_casements_shadow() {
    for renderer in RENDERERS {
        let shots = shoot(
            renderer,
            Say::Frame(FrameMode::CompositorShadow),
            Buffer::Whole,
            Placed::Floating,
        );
        let (left, top, right, bottom) = extent(&shots.shot).expect("the window is on screen");
        assert_eq!((right - left + 1, bottom - top + 1), SIZE, "{renderer:?}");
        assert_round(&shots.shot, (left, top, right, bottom), renderer);
        assert_shadow(&shots, (left, top, right, bottom), renderer);
    }
}

#[test]
fn a_window_that_says_nothing_and_fills_its_buffer_gets_the_desktops_corners_and_a_shadow() {
    for renderer in RENDERERS {
        let shots = shoot(renderer, Say::Nothing, Buffer::Whole, Placed::Floating);
        let found = extent(&shots.shot).expect("the window is on screen");
        assert_round(&shots.shot, found, renderer);
        assert_shadow(&shots, found, renderer);
    }
}

#[test]
fn a_window_with_a_margin_in_its_buffer_is_clipped_at_its_geometry_and_gets_no_shadow() {
    for renderer in RENDERERS {
        let shots = shoot(renderer, Say::Nothing, Buffer::Margin, Placed::Floating);
        let (left, top, right, bottom) = extent(&shots.shot).expect("the window is on screen");
        // The buffer is whole (the margin is left alone), and the geometry's corners are cut.
        let grown = (SIZE.0 + 2 * MARGIN, SIZE.1 + 2 * MARGIN);
        assert_eq!((right - left + 1, bottom - top + 1), grown, "{renderer:?}");
        let geometry = (left + MARGIN, top + MARGIN, right - MARGIN, bottom - MARGIN);
        assert_round(&shots.shot, geometry, renderer);
        assert_eq!(
            shots.shot.get_pixel(left - 4, (top + bottom) / 2),
            shots.bare.get_pixel(left - 4, (top + bottom) / 2),
            "casement adds no shadow beside the toolkit's own on {renderer:?}"
        );
    }
}

#[test]
fn a_window_that_says_it_draws_its_own_frame_is_left_square_and_unshadowed() {
    for renderer in RENDERERS {
        let shots = shoot(
            renderer,
            Say::Frame(FrameMode::Client),
            Buffer::Whole,
            Placed::Floating,
        );
        let (left, top, right, bottom) = extent(&shots.shot).expect("the window is on screen");
        for (name, x, y) in [
            ("top left", left, top),
            ("top right", right, top),
            ("bottom left", left, bottom),
            ("bottom right", right, bottom),
        ] {
            let pixel = shots.shot.get_pixel(x, y).0;
            assert!(is_client(pixel), "{name} on {renderer:?}");
        }
        let (x, y) = (left - 4, (top + bottom) / 2);
        assert_eq!(
            shots.shot.get_pixel(x, y),
            shots.bare.get_pixel(x, y),
            "no shadow on {renderer:?}"
        );
    }
}

#[test]
fn a_full_screen_window_stays_square_whatever_it_asked() {
    for renderer in RENDERERS {
        for say in [Say::Frame(FrameMode::CompositorShadow), Say::Nothing] {
            let shots = shoot(renderer, say, Buffer::Whole, Placed::FullScreen);
            let (left, top, right, bottom) = extent(&shots.shot).expect("the window is on screen");
            assert_eq!(
                (left, top, right + 1, bottom + 1),
                (0, 0, shots.shot.width(), shots.shot.height()),
                "it covers the output on {renderer:?}"
            );
            for (name, x, y) in [
                ("top left", left, top),
                ("top right", right, top),
                ("bottom left", left, bottom),
                ("bottom right", right, bottom),
            ] {
                assert!(
                    is_client(shots.shot.get_pixel(x, y).0),
                    "{name} of {say:?} on {renderer:?}"
                );
            }
        }
    }
}

/// The desktop with and without one 400x300 window of `buffer`, told `say`, placed as `placed`,
/// drawn by `renderer`.
fn shoot(renderer: SoftRenderer, say: Say, buffer: Buffer, placed: Placed) -> Shots {
    let spec = FixtureSpec::desktop().with_renderer(renderer);
    let mut fixture = Fixture::start(spec).expect("fixture");
    let mut desk = Desk::at(&mut fixture);
    desk.wallpaper(WALLPAPER, Dpi::One);
    let bare = desk.shoot();
    let grown = match buffer {
        Buffer::Whole => SIZE,
        Buffer::Margin => (SIZE.0 + 2 * MARGIN, SIZE.1 + 2 * MARGIN),
    };
    let top = desk.window(APP, grown, Pattern::Solid(CLIENT), Dpi::One);
    if let Buffer::Margin = buffer {
        let side = |n: u32| i32::try_from(n).expect("a small side");
        top.xdg_surface
            .set_window_geometry(side(MARGIN), side(MARGIN), side(SIZE.0), side(SIZE.1));
    }
    let manager = WindowManager::bind(desk.fixture.client(desk.id), 1);
    let ext = manager.window(desk.fixture.client(desk.id), &top);
    if let Say::Frame(mode) = say {
        ext.set_frame(mode, RADIUS);
    }
    top.surface.commit();
    desk.fixture.pump();
    if let Placed::FullScreen = placed {
        // Placed first, as the person's windows are: full screen leaves from a place.
        let window = crate::scene::desk::only_window(desk.fixture);
        let at = casement_core::geometry::LogicalPoint { x: 100, y: 100 };
        desk.fixture.place_window(window, at);
        // Only the key window goes full screen: a click inside makes it key.
        let device = desk.fixture.input().plug(
            casement_core::input::DeviceClass::Pointer,
            casement_testkit::TestOrigin::Physical(casement_core::input::DeviceClass::Pointer),
        );
        let inside = casement_core::geometry::LogicalPoint { x: 300, y: 250 };
        desk.fixture.input().pointer_to(device, inside);
        for edge in [Edge::Press, Edge::Release] {
            desk.fixture.input().button(device, ButtonCode(0x110), edge);
        }
        desk.fixture.pump();
        top.set_fullscreen();
        top.commit_bare();
        desk.fixture.pump();
        assert!(
            top.ack_latest().is_some(),
            "the full-screen configure arrives"
        );
        // The output at 1x, as the bare shot is.
        let screen = casement_core::geometry::PhysicalSize {
            w: bare.width(),
            h: bare.height(),
        };
        let covering = casement_testclient::shm_buffer(
            desk.fixture.client(desk.id),
            screen,
            Pattern::Solid(CLIENT),
        );
        top.show(&covering);
        desk.fixture.pump();
        desk.kept.keep(covering);
    }
    desk.kept.keep((manager, ext, top));
    let shot = desk.shoot();
    Shots { bare, shot }
}

/// The four corner pixels of the rectangle `(left, top, right, bottom)` (inclusive) are not the
/// window's colour, and a pixel `RADIUS + 2` in from each is.
fn assert_round(shot: &image::RgbaImage, found: Rect, renderer: SoftRenderer) {
    let (left, top, right, bottom) = found;
    let inset = RADIUS + 2;
    for (name, x, y, want) in [
        ("top left corner", left, top, false),
        ("top right corner", right, top, false),
        ("bottom left corner", left, bottom, false),
        ("bottom right corner", right, bottom, false),
        ("inside the top left", left + inset, top + inset, true),
        (
            "inside the bottom right",
            right - inset,
            bottom - inset,
            true,
        ),
        ("top edge beside the curve", left + inset, top, true),
        ("left edge beside the curve", left, top + inset, true),
    ] {
        assert_eq!(
            is_client(shot.get_pixel(x, y).0),
            want,
            "{name} at ({x}, {y}) on {renderer:?}"
        );
    }
}

/// The pixel just outside the middle of each side is darker than the wallpaper there: the
/// shadow falls round the window.
fn assert_shadow(shots: &Shots, found: Rect, renderer: SoftRenderer) {
    let (left, top, right, bottom) = found;
    let (middle_x, middle_y) = ((left + right) / 2, (top + bottom) / 2);
    for (name, x, y) in [
        ("left", left - 4, middle_y),
        ("right", right + 4, middle_y),
        ("below", middle_x, bottom + 4),
    ] {
        let (lit, dark) = (shots.bare.get_pixel(x, y).0, shots.shot.get_pixel(x, y).0);
        assert!(
            luma(dark) + 3 < luma(lit),
            "the shadow darkens the pixel {name} on {renderer:?}: {dark:?} against {lit:?}"
        );
    }
}

/// A pixel's brightness, 0 to 255.
fn luma(pixel: [u8; 4]) -> i32 {
    (i32::from(pixel[0]) * 299 + i32::from(pixel[1]) * 587 + i32::from(pixel[2]) * 114) / 1000
}

/// Whether `pixel` is the window's colour.
fn is_client(pixel: [u8; 4]) -> bool {
    pixel
        .iter()
        .zip(CLIENT_RGBA)
        .all(|(a, b)| a.abs_diff(b) <= 1)
}

/// The left, top, right and bottom pixel of the window's colour in `shot`.
fn extent(shot: &image::RgbaImage) -> Option<Rect> {
    shot.enumerate_pixels()
        .filter(|(_, _, pixel)| is_client(pixel.0))
        .fold(None, |found, (x, y, _)| match found {
            None => Some((x, y, x, y)),
            Some((l, t, r, b)) => Some((l.min(x), t.min(y), r.max(x), b.max(y))),
        })
}
