//! The cursor in a capture (wave-2026-10-15/capture3): a cursor a client drew itself is a frame
//! of the cursor session, a turned output sends the cursor turned with its hotspot, and at a
//! fractional scale the capture puts the cursor where the screen does.

use crate::protocol::data_device::{focused, map};
use crate::protocol::image_capture::{cursor_watch, name, pointer_at, settle};
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::policy::ConnectionRole;
use casement_testclient::capture::{BufferAsk, CursorEvent, CursorMode, OutputCapture, Size};
use casement_testclient::misc::viewporter::Viewporter;
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::surface::layer::Layer;
use casement_testclient::surface::subsurface::Surface;
use casement_testclient::{Pattern, Pumped, shm_buffer};
use casement_testkit::{Fixture, FixtureSpec, FrameCount};
use wayland_client::protocol::wl_output::Transform;

/// Where the pointer rests: over the middle of the layer the client maps.
const REST: (i32, i32) = (960, 540);

/// A cursor session's image, as RGBA, after it was asked for once.
fn image_of(fixture: &mut Fixture, capture: &mut OutputCapture) -> (Size, Vec<u8>) {
    capture.request(BufferAsk::Announced).expect("a frame");
    settle(fixture, capture);
    let image = capture
        .take()
        .expect("the image was answered")
        .expect("a frame, not a failure");
    let rgba = image.to_rgba8().expect("rgba");
    (image.size, rgba)
}

/// A cursor surface the client attached, committed and set as the pointer's cursor is a frame
/// of the cursor session: its own pixels, at its own size, with its own hotspot.
#[test]
fn a_cursor_the_client_drew_is_a_frame_of_its_own() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = pointer_at(&mut fixture, (10, 10));
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let _layer = map(&mut fixture, id, &focused(Layer::Top, "canvas"));
    fixture.input().pointer_to(
        mouse,
        LogicalPoint {
            x: REST.0,
            y: REST.1,
        },
    );
    fixture.pump();
    let serial = pointer
        .enter_serial()
        .expect("the pointer entered the layer");

    let surface = fixture.client(id).create_surface().expect("a surface");
    let art = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 16, h: 16 },
        Pattern::Solid(0xff33_6699),
    );
    surface.attach(Some(art.buffer()), 0, 0);
    surface.commit();
    pointer.pointer().set_cursor(serial, Some(&surface), 4, 4);
    fixture.pump();
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));

    let mut capture = OutputCapture::start(fixture.client(id).globals()).expect("the managers");
    settle(&mut fixture, &mut capture);
    capture.open_cursor("HEADLESS-1").expect("a cursor session");
    settle(&mut fixture, &mut capture);

    let session_size = capture.session().and_then(|session| session.size);
    assert_eq!(session_size, Some(Size::new(16, 16)), "the surface's size");
    assert_eq!(
        capture.take_cursor_events(),
        [
            CursorEvent::Entered,
            CursorEvent::Hotspot(4, 4),
            CursorEvent::Position(REST.0, REST.1)
        ],
        "the client's hotspot, at the pointer"
    );
    let (size, rgba) = image_of(&mut fixture, &mut capture);
    assert_eq!((size.w, size.h), (16, 16));
    assert!(
        rgba.chunks(4)
            .all(|pixel| pixel == [0x33, 0x66, 0x99, 0xff]),
        "the client's own pixels"
    );
}

/// Each transform: the events the cursor session sends for a pointer at (100, 50) of a
/// 640 x 480 output, the hotspot turned inside the 24 x 24 arrow, and where one of the arrow's
/// pixels (x, y) lands in the turned image.
type Turn = (
    &'static str,
    (i32, i32),
    (i32, i32),
    fn(i32, i32) -> (i32, i32),
);

const TURNS: [Turn; 8] = [
    ("normal", (100, 50), (5, 3), |x, y| (x, y)),
    ("90", (590, 100), (21, 5), |x, y| (23 - y, x)),
    ("180", (540, 430), (19, 21), |x, y| (23 - x, 23 - y)),
    ("270", (50, 380), (3, 19), |x, y| (y, 23 - x)),
    ("flipped", (540, 50), (19, 3), |x, y| (23 - x, y)),
    ("flipped-90", (50, 100), (3, 5), |x, y| (y, x)),
    ("flipped-180", (100, 430), (5, 21), |x, y| (x, 23 - y)),
    ("flipped-270", (590, 380), (21, 19), |x, y| (23 - y, 23 - x)),
];

/// A turned output sends the cursor image turned with the frame and the hotspot turned to
/// match: the position minus the hotspot is where the turned image's corner is.
#[test]
fn a_turned_output_sends_the_cursor_turned() {
    let straight = {
        let spec = FixtureSpec::desktop().with_outputs(&["HEADLESS-1:640x480@60000:120:0,0"]);
        let (mut fixture, mut capture, _pointer) = cursor_watch(spec, "HEADLESS-1", (100, 50));
        image_of(&mut fixture, &mut capture).1
    };
    for (word, position, hotspot, moved) in TURNS {
        let output = format!("HEADLESS-1:640x480@60000:120:0,0:transform={word}");
        let spec = FixtureSpec::desktop().with_outputs(&[output.as_str()]);
        let (mut fixture, mut capture, _pointer) = cursor_watch(spec, "HEADLESS-1", (100, 50));
        assert_eq!(
            capture.take_cursor_events(),
            [
                CursorEvent::Entered,
                CursorEvent::Hotspot(hotspot.0, hotspot.1),
                CursorEvent::Position(position.0, position.1)
            ],
            "{word}"
        );
        let (size, turned) = image_of(&mut fixture, &mut capture);
        assert_eq!((size.w, size.h), (24, 24), "{word}");
        for y in 0..24_i32 {
            for x in 0..24_i32 {
                let (tx, ty) = moved(x, y);
                let from = usize::try_from((y * 24 + x) * 4).expect("small");
                let to = usize::try_from((ty * 24 + tx) * 4).expect("small");
                assert_eq!(
                    turned[to..to + 4],
                    straight[from..from + 4],
                    "{word}: pixel ({x}, {y}) moved to ({tx}, {ty})"
                );
            }
        }
    }
}

/// Pumps the compositor and both captures until nothing moves.
fn settle_both(fixture: &mut Fixture, a: &mut OutputCapture, b: &mut OutputCapture) {
    for _ in 0..100 {
        let compositor = fixture.pump();
        let first = a.pump().expect("pump the capture");
        let second = b.pump().expect("pump the cursor session");
        if (compositor, first, second) == (Pumped::Quiet, Pumped::Quiet, Pumped::Quiet) {
            return;
        }
    }
    panic!("the compositor and the captures did not go quiet");
}

/// At 1.5x the cursor the capture paints and the cursor session places agree to the pixel:
/// every opaque pixel of the session's image is in the painted frame at the position the
/// session gave, minus its hotspot. (The pointer is at (100, 50), where rounding the hotspot
/// and the position separately lands one pixel off the element.)
#[test]
fn the_capture_and_the_screen_agree_at_a_fractional_scale() {
    let spec = FixtureSpec::desktop().with_outputs(&["HEADLESS-1:960x720@60000:180:0,0"]);
    let mut fixture = Fixture::start(spec).expect("fixture");
    let _mouse = pointer_at(&mut fixture, (100, 50));
    let id = fixture.connect(ConnectionRole::Public);
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut painted = OutputCapture::start(fixture.client(id).globals()).expect("the managers");
    let mut cursor = OutputCapture::start(fixture.client(id).globals()).expect("the managers");
    settle_both(&mut fixture, &mut painted, &mut cursor);
    painted
        .open("HEADLESS-1", CursorMode::Painted)
        .expect("a session");
    cursor.open_cursor("HEADLESS-1").expect("a cursor session");
    settle_both(&mut fixture, &mut painted, &mut cursor);

    let events = cursor.take_cursor_events();
    let (Some(CursorEvent::Hotspot(hx, hy)), Some(CursorEvent::Position(x, y))) =
        (events.get(1).copied(), events.get(2).copied())
    else {
        panic!("a hotspot and a position, got {events:?}");
    };
    let corner = (x - hx, y - hy);
    assert_eq!(corner, (143, 71), "round(95 x 1.5), round(47 x 1.5)");

    cursor.request(BufferAsk::Announced).expect("an image");
    painted.request(BufferAsk::Announced).expect("a frame");
    settle_both(&mut fixture, &mut painted, &mut cursor);
    let image = cursor.take().expect("answered").expect("an image");
    let frame = painted.take().expect("answered").expect("a frame");
    let (image_rgba, frame_rgba) = (
        image.to_rgba8().expect("rgba"),
        frame.to_rgba8().expect("rgba"),
    );
    let (iw, fw) = (
        usize::try_from(image.size.w).expect("small"),
        usize::try_from(frame.size.w).expect("small"),
    );
    let mut opaque = 0;
    for (n, pixel) in image_rgba.chunks(4).enumerate() {
        if pixel[3] != 255 {
            continue;
        }
        opaque += 1;
        let at = (
            usize::try_from(corner.0).expect("on the frame") + n % iw,
            usize::try_from(corner.1).expect("on the frame") + n / iw,
        );
        let from = (at.1 * fw + at.0) * 4;
        assert_eq!(
            frame_rgba[from..from + 4],
            *pixel,
            "image pixel {n} at {at:?} of the frame"
        );
    }
    assert!(
        opaque > 20,
        "the arrow has an opaque body, found {opaque} pixels"
    );
}

/// How a client dresses its 16 x 16 cursor surface before it commits.
enum Dress {
    /// The viewport takes the buffer's (4, 2) 8 x 8 part and shows it 16 x 16.
    Viewport,
    /// The buffer is declared turned half a circle.
    HalfTurn,
}

/// The image a cursor session sends for a coordinate-encoded 16 x 16 cursor buffer (pixel
/// (x, y) is red = x, green = y) that the client dressed with `dress`.
fn dressed_cursor(dress: &Dress) -> (Size, Vec<u8>) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = pointer_at(&mut fixture, (10, 10));
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let _layer = map(&mut fixture, id, &focused(Layer::Top, "canvas"));
    fixture.input().pointer_to(
        mouse,
        LogicalPoint {
            x: REST.0,
            y: REST.1,
        },
    );
    fixture.pump();
    let serial = pointer
        .enter_serial()
        .expect("the pointer entered the layer");
    let surface = fixture.client(id).create_surface().expect("a surface");
    let art = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 16, h: 16 },
        Pattern::CoordinateEncoding,
    );
    match dress {
        Dress::Viewport => {
            let viewporter = Viewporter::bind(fixture.client(id));
            let wrapped = Surface {
                surface: surface.clone(),
            };
            let viewport = viewporter.viewport(fixture.client(id), &wrapped);
            viewport.source(4.0, 2.0, 8.0, 8.0);
            viewport.destination(16, 16);
        }
        Dress::HalfTurn => surface.set_buffer_transform(Transform::_180),
    }
    surface.attach(Some(art.buffer()), 0, 0);
    surface.commit();
    pointer.pointer().set_cursor(serial, Some(&surface), 0, 0);
    fixture.pump();
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut capture = OutputCapture::start(fixture.client(id).globals()).expect("the managers");
    settle(&mut fixture, &mut capture);
    capture.open_cursor("HEADLESS-1").expect("a cursor session");
    settle(&mut fixture, &mut capture);
    image_of(&mut fixture, &mut capture)
}

/// A cursor with a viewport is captured as drawn: the cropped part, scaled to the shown size.
#[test]
fn a_cursor_with_a_viewport_is_captured_as_drawn() {
    let (size, rgba) = dressed_cursor(&Dress::Viewport);
    assert_eq!((size.w, size.h), (16, 16), "the viewport's destination");
    for y in 0..16_u8 {
        for x in 0..16_u8 {
            let at = (usize::from(y) * 16 + usize::from(x)) * 4;
            assert_eq!(
                rgba[at..at + 2],
                [4 + x / 2, 2 + y / 2],
                "pixel ({x}, {y}) comes from the buffer's ({}, {})",
                4 + x / 2,
                2 + y / 2
            );
        }
    }
}

/// A cursor whose buffer is declared turned is captured upright, as the screen shows it.
#[test]
fn a_cursor_with_a_buffer_transform_is_captured_upright() {
    let (size, rgba) = dressed_cursor(&Dress::HalfTurn);
    assert_eq!((size.w, size.h), (16, 16));
    for y in 0..16_u8 {
        for x in 0..16_u8 {
            let at = (usize::from(y) * 16 + usize::from(x)) * 4;
            assert_eq!(
                rgba[at..at + 2],
                [15 - x, 15 - y],
                "pixel ({x}, {y}) of the half-turned buffer"
            );
        }
    }
}
