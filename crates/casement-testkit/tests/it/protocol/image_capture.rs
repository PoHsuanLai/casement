//! `image_capture` (P1-C1): output capture through ext-image-copy-capture, read by
//! `casement_testclient::capture` (which folds the events with `desktop_client`'s reducer).

use casement_core::geometry::LogicalPoint;
use casement_core::input::DeviceClass;
use casement_core::policy::ConnectionRole;
use casement_core::{DeviceId, OutputName};
use casement_testclient::Pumped;
use casement_testclient::capture::{
    BufferAsk, CaptureFailure, CursorEvent, CursorMode, OutputCapture, Size,
};
use casement_testkit::{ClientId, Fixture, FixtureSpec, FrameCount, ShotTarget, TestOrigin};

/// One small output, so a frame is a few hundred kilobytes.
pub(crate) fn small() -> FixtureSpec {
    FixtureSpec::desktop().with_outputs(&["HEADLESS-1:640x480@60000:120:0,0"])
}

pub(crate) fn name(text: &str) -> OutputName {
    OutputName::parse(text).expect("a name")
}

/// Pumps the compositor and the capture until neither moves.
pub(crate) fn settle(fixture: &mut Fixture, capture: &mut OutputCapture) {
    for _ in 0..100 {
        let compositor = fixture.pump();
        let client = capture.pump().expect("pump the capture");
        if (compositor, client) == (Pumped::Quiet, Pumped::Quiet) {
            return;
        }
    }
    panic!("the compositor and the capture did not go quiet");
}

/// A fixture with a client, two presented frames and a capture of `output` opened and
/// settled (its constraints are in), with the cursor painted or not.
pub(crate) fn opened_with(
    spec: FixtureSpec,
    output: &str,
    cursor: CursorMode,
) -> (Fixture, ClientId, OutputCapture) {
    let mut fixture = Fixture::start(spec).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut capture = OutputCapture::start(fixture.client(client).globals()).expect("the managers");
    settle(&mut fixture, &mut capture);
    capture
        .open(output, cursor)
        .expect("a session on the output");
    settle(&mut fixture, &mut capture);
    (fixture, client, capture)
}

/// A capture that does not ask for the cursor.
pub(crate) fn opened(spec: FixtureSpec, output: &str) -> (Fixture, ClientId, OutputCapture) {
    opened_with(spec, output, CursorMode::Hidden)
}

/// The captured frame equals the frame scanout shows (cursor and all, when it is painted), byte
/// for byte on pixman.
#[test]
fn a_capture_equals_the_scanout_frame() {
    let (mut fixture, _client, mut capture) =
        opened_with(small(), "HEADLESS-1", CursorMode::Painted);
    let session = capture.session().expect("a session");
    assert_eq!(session.size, Some(Size::new(640, 480)));
    capture.request(BufferAsk::Announced).expect("a frame");
    settle(&mut fixture, &mut capture);
    let frame = capture
        .take()
        .expect("the frame was answered")
        .expect("a frame, not a failure");
    let scanout = fixture.shot(ShotTarget::Output(name("HEADLESS-1")));
    assert_eq!(
        (frame.size.w, frame.size.h),
        (scanout.width(), scanout.height())
    );
    assert_eq!(
        frame.to_rgba8().expect("argb8888 reads as rgba"),
        scanout.into_raw(),
        "capture equals scanout"
    );
}

/// The session announces the buffer constraints; a buffer that does not match is `failed`.
#[test]
fn a_mismatched_buffer_fails_the_frame() {
    let (mut fixture, _client, mut capture) = opened(small(), "HEADLESS-1");
    capture
        .request(BufferAsk::Sized(Size::new(320, 240)))
        .expect("a request");
    settle(&mut fixture, &mut capture);
    assert_eq!(
        capture.take().expect("answered").expect_err("a failure"),
        CaptureFailure::BufferConstraints
    );
    assert!(
        !capture.is_stopped(),
        "a bad buffer does not end the session"
    );
}

/// An unplugged output stops its sessions.
#[test]
fn an_unplugged_output_stops_its_sessions() {
    let spec = FixtureSpec::desktop().with_outputs(&[
        "HEADLESS-1:640x480@60000:120:0,0",
        "HEADLESS-2:640x480@60000:120:640,0",
    ]);
    let (mut fixture, _client, mut capture) = opened(spec, "HEADLESS-2");
    assert!(!capture.is_stopped());
    fixture.unplug(&name("HEADLESS-2"));
    settle(&mut fixture, &mut capture);
    assert!(capture.is_stopped(), "the session was told `stopped`");
}

/// Plugs a pointer and puts it at `at` (logical pixels), before any client connects.
pub(crate) fn pointer_at(fixture: &mut Fixture, at: (i32, i32)) -> DeviceId {
    let device = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    move_pointer(fixture, device, at);
    device
}

/// Moves the pointer of `device` to `at` (logical pixels).
pub(crate) fn move_pointer(fixture: &mut Fixture, device: DeviceId, at: (i32, i32)) {
    fixture
        .input()
        .pointer_to(device, LogicalPoint { x: at.0, y: at.1 });
}

/// Whether two RGBA images of `width` pixels agree everywhere outside `skip`
/// (`x0, y0, x1, y1`, the right and bottom edges excluded).
fn agree_outside(a: &[u8], b: &[u8], width: usize, skip: (usize, usize, usize, usize)) -> bool {
    a.chunks(4).zip(b.chunks(4)).enumerate().all(|(n, (p, q))| {
        let (x, y) = (n % width, n / width);
        let inside = x >= skip.0 && x < skip.2 && y >= skip.1 && y < skip.3;
        inside || p == q
    })
}

/// The scanout frame and the capture a session made of it, the pointer at (300, 200).
fn scanout_and_capture(cursor: CursorMode) -> (Vec<u8>, Vec<u8>) {
    let mut fixture = Fixture::start(small()).expect("fixture");
    let _pointer = pointer_at(&mut fixture, (300, 200));
    let client = fixture.connect(ConnectionRole::Public);
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut capture = OutputCapture::start(fixture.client(client).globals()).expect("the managers");
    settle(&mut fixture, &mut capture);
    capture.open("HEADLESS-1", cursor).expect("a session");
    settle(&mut fixture, &mut capture);
    capture.request(BufferAsk::Announced).expect("a frame");
    settle(&mut fixture, &mut capture);
    let frame = capture
        .take()
        .expect("the frame was answered")
        .expect("a frame, not a failure");
    let scanout = fixture.shot(ShotTarget::Output(name("HEADLESS-1")));
    (scanout.into_raw(), frame.to_rgba8().expect("rgba"))
}

/// The cursor is in a capture when the session asks for it (`paint_cursors`) and nowhere else
/// in the frame changes when it does not.
#[test]
fn the_cursor_is_in_a_capture_only_when_asked_for() {
    // The arrow's canvas is 24 pixels and its hotspot (5, 3): it covers 295..319 x 197..221.
    let cursor_box = (290, 190, 325, 225);
    let (scanout, painted) = scanout_and_capture(CursorMode::Painted);
    assert_eq!(painted, scanout, "painted: the scanout frame");
    let (scanout, hidden) = scanout_and_capture(CursorMode::Hidden);
    assert_ne!(hidden, scanout, "hidden: the cursor is gone from the frame");
    assert!(
        agree_outside(&hidden, &scanout, 640, cursor_box),
        "hidden: only the cursor's pixels differ from the scanout"
    );
}

/// After its first frame a session's next frame waits for something to change, and then says
/// what.
#[test]
fn a_second_frame_waits_for_a_change_and_reports_it() {
    let mut fixture = Fixture::start(small()).expect("fixture");
    let pointer = pointer_at(&mut fixture, (500, 400));
    let client = fixture.connect(ConnectionRole::Public);
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut capture = OutputCapture::start(fixture.client(client).globals()).expect("the managers");
    settle(&mut fixture, &mut capture);
    capture
        .open("HEADLESS-1", CursorMode::Painted)
        .expect("a session");
    settle(&mut fixture, &mut capture);
    capture.request(BufferAsk::Announced).expect("a frame");
    settle(&mut fixture, &mut capture);
    let first = capture.take().expect("answered").expect("a frame");
    assert_eq!(first.damage.len(), 1, "the first frame is whole");
    assert_eq!((first.damage[0].w, first.damage[0].h), (640, 480));

    // Nothing moves: frames pass and the request stays.
    capture.request(BufferAsk::Announced).expect("a frame");
    fixture.frames(&name("HEADLESS-1"), FrameCount(3));
    settle(&mut fixture, &mut capture);
    assert!(capture.take().is_none(), "no change, no frame");

    // The pointer moves: the frame comes, with the old and the new place of the cursor.
    move_pointer(&mut fixture, pointer, (100, 100));
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    settle(&mut fixture, &mut capture);
    let second = capture.take().expect("answered").expect("a frame");
    assert!(!second.damage.is_empty(), "something changed");
    let whole = 640 * 480;
    assert!(
        second.damage.iter().all(|rect| rect.w * rect.h < whole / 4),
        "only small parts changed: {:?}",
        second.damage
    );
    let covers = |x: i32, y: i32| {
        second.damage.iter().any(|rect| {
            let w = i32::try_from(rect.w).unwrap_or(0);
            let h = i32::try_from(rect.h).unwrap_or(0);
            rect.x <= x && x < rect.x + w && rect.y <= y && y < rect.y + h
        })
    };
    assert!(covers(100, 100), "the new place of the cursor");
}

/// Pumps the compositor and every capture until none of them moves.
fn settle_all(fixture: &mut Fixture, captures: &mut [&mut OutputCapture]) {
    for _ in 0..100 {
        let mut pumped = fixture.pump();
        for capture in captures.iter_mut() {
            if capture.pump().expect("pump the capture") != Pumped::Quiet {
                pumped = Pumped::Moved;
            }
        }
        if pumped == Pumped::Quiet {
            return;
        }
    }
    panic!("the compositor and the captures did not go quiet");
}

/// A screen share keeps one buffer and asks frame after frame on one session: each frame is
/// destroyed and the next created with the same buffer. Every one is answered, none ends the
/// session or the connection, and the last equals the scanout.
#[test]
fn a_session_serves_frame_after_frame_into_one_buffer() {
    let mut fixture = Fixture::start(small()).expect("fixture");
    let pointer = pointer_at(&mut fixture, (500, 400));
    let client = fixture.connect(ConnectionRole::Public);
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut capture = OutputCapture::start(fixture.client(client).globals()).expect("the managers");
    settle(&mut fixture, &mut capture);
    capture
        .open("HEADLESS-1", CursorMode::Painted)
        .expect("a session");
    settle(&mut fixture, &mut capture);

    capture.request(BufferAsk::Announced).expect("a frame");
    settle(&mut fixture, &mut capture);
    let mut last = capture.take().expect("answered").expect("a frame");
    assert_eq!(last.damage.len(), 1, "the first frame is whole");

    for at in [(100, 100), (300, 250), (500, 400)] {
        capture.request(BufferAsk::Reuse).expect("the same buffer");
        move_pointer(&mut fixture, pointer, at);
        fixture.frames(&name("HEADLESS-1"), FrameCount(2));
        settle(&mut fixture, &mut capture);
        last = capture
            .take()
            .unwrap_or_else(|| panic!("no frame after the pointer went to {at:?}"))
            .expect("a frame, not a failure");
        assert!(!last.damage.is_empty(), "something changed at {at:?}");
        assert!(!capture.is_stopped(), "the session goes on");
    }
    let scanout = fixture.shot(ShotTarget::Output(name("HEADLESS-1")));
    assert_eq!(
        last.to_rgba8().expect("argb8888 reads as rgba"),
        scanout.into_raw(),
        "the last frame equals the scanout"
    );
}

/// A frame that waits for a change (a still desktop sends none) leaves the client connected and
/// the compositor serving: a second session, opened while the first still waits, gets its own
/// whole first frame, and both are answered when something moves. Closing the first leaves the
/// second working.
#[test]
fn a_waiting_frame_does_not_hold_up_a_second_session() {
    let mut fixture = Fixture::start(small()).expect("fixture");
    let pointer = pointer_at(&mut fixture, (500, 400));
    let client = fixture.connect(ConnectionRole::Public);
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut first = OutputCapture::start(fixture.client(client).globals()).expect("the managers");
    settle_all(&mut fixture, &mut [&mut first]);
    first
        .open("HEADLESS-1", CursorMode::Painted)
        .expect("a session");
    settle_all(&mut fixture, &mut [&mut first]);
    first.request(BufferAsk::Announced).expect("a frame");
    settle_all(&mut fixture, &mut [&mut first]);
    first.take().expect("answered").expect("a frame");

    // The still desktop: the first session's next frame waits, and frames pass over it.
    first.request(BufferAsk::Reuse).expect("the same buffer");
    fixture.frames(&name("HEADLESS-1"), FrameCount(3));
    settle_all(&mut fixture, &mut [&mut first]);
    assert!(first.take().is_none(), "no change, no frame");
    assert!(!first.is_stopped(), "waiting does not end the session");

    let mut second = OutputCapture::start(fixture.client(client).globals()).expect("the managers");
    settle_all(&mut fixture, &mut [&mut first, &mut second]);
    second
        .open("HEADLESS-1", CursorMode::Painted)
        .expect("a second session");
    settle_all(&mut fixture, &mut [&mut first, &mut second]);
    second.request(BufferAsk::Announced).expect("a frame");
    settle_all(&mut fixture, &mut [&mut first, &mut second]);
    let whole = second.take().expect("answered").expect("a frame");
    assert_eq!(whole.damage.len(), 1, "a session's first frame is whole");
    assert_eq!((whole.damage[0].w, whole.damage[0].h), (640, 480));
    assert!(first.take().is_none(), "the first still waits");

    second.request(BufferAsk::Reuse).expect("the same buffer");
    move_pointer(&mut fixture, pointer, (100, 100));
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    settle_all(&mut fixture, &mut [&mut first, &mut second]);
    first
        .take()
        .expect("the first session was answered once something moved")
        .expect("a frame, not a failure");
    second
        .take()
        .expect("the second session was answered")
        .expect("a frame, not a failure");

    first.close().expect("close the first session");
    settle_all(&mut fixture, &mut [&mut first, &mut second]);
    second.request(BufferAsk::Reuse).expect("the same buffer");
    move_pointer(&mut fixture, pointer, (300, 250));
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    settle_all(&mut fixture, &mut [&mut first, &mut second]);
    second
        .take()
        .expect("the second session goes on after the first closed")
        .expect("a frame, not a failure");
    assert!(!second.is_stopped());
}

/// Two outputs side by side, a cursor session on the second.
fn two_outputs(scale: u32) -> FixtureSpec {
    let first = format!("HEADLESS-1:640x480@60000:{scale}:0,0");
    let second = format!("HEADLESS-2:640x480@60000:{scale}:640,0");
    FixtureSpec::desktop().with_outputs(&[first.as_str(), second.as_str()])
}

/// A cursor session on `output` of a fixture whose pointer is at `at`; the session's first
/// events are in.
pub(crate) fn cursor_watch(
    spec: FixtureSpec,
    output: &str,
    at: (i32, i32),
) -> (Fixture, OutputCapture, DeviceId) {
    let mut fixture = Fixture::start(spec).expect("fixture");
    let pointer = pointer_at(&mut fixture, at);
    let client = fixture.connect(ConnectionRole::Public);
    fixture.frames(&name(output), FrameCount(2));
    let mut capture = OutputCapture::start(fixture.client(client).globals()).expect("the managers");
    settle(&mut fixture, &mut capture);
    capture.open_cursor(output).expect("a cursor session");
    settle(&mut fixture, &mut capture);
    (fixture, capture, pointer)
}

/// The cursor enters when its image meets the output, is placed in the output's pixels, moves
/// with the pointer and leaves with it.
#[test]
fn a_cursor_session_follows_the_pointer_on_its_output() {
    let (mut fixture, mut capture, pointer) =
        cursor_watch(two_outputs(120), "HEADLESS-2", (300, 200));
    assert!(
        capture.take_cursor_events().is_empty(),
        "the pointer is on the other output"
    );

    move_pointer(&mut fixture, pointer, (700, 200));
    fixture.frames(&name("HEADLESS-2"), FrameCount(1));
    settle(&mut fixture, &mut capture);
    assert_eq!(
        capture.take_cursor_events(),
        [
            CursorEvent::Entered,
            CursorEvent::Hotspot(5, 3),
            CursorEvent::Position(60, 200)
        ]
    );

    move_pointer(&mut fixture, pointer, (710, 220));
    fixture.frames(&name("HEADLESS-2"), FrameCount(1));
    settle(&mut fixture, &mut capture);
    assert_eq!(
        capture.take_cursor_events(),
        [CursorEvent::Position(70, 220)]
    );

    move_pointer(&mut fixture, pointer, (300, 200));
    fixture.frames(&name("HEADLESS-2"), FrameCount(1));
    settle(&mut fixture, &mut capture);
    assert_eq!(capture.take_cursor_events(), [CursorEvent::Left]);
}

/// At 1.5x the position and the hotspot are in the output's physical pixels.
#[test]
fn a_cursor_session_speaks_physical_pixels() {
    let spec = FixtureSpec::desktop().with_outputs(&["HEADLESS-1:960x720@60000:180:0,0"]);
    let (_fixture, mut capture, _pointer) = cursor_watch(spec, "HEADLESS-1", (100, 50));
    assert_eq!(
        capture.take_cursor_events(),
        [
            CursorEvent::Entered,
            CursorEvent::Hotspot(8, 5),
            CursorEvent::Position(151, 76)
        ]
    );
    let size = capture.session().and_then(|session| session.size);
    assert_eq!(
        size,
        Some(Size::new(36, 36)),
        "the image is 24 logical pixels at 1.5x"
    );
}

/// The cursor's image is a capture source of its own: its pixels once, and a new frame only
/// when they change.
#[test]
fn the_cursor_image_is_a_capture_of_its_own() {
    let (mut fixture, mut capture, _pointer) = cursor_watch(small(), "HEADLESS-1", (300, 200));
    capture.request(BufferAsk::Announced).expect("a frame");
    settle(&mut fixture, &mut capture);
    let image = capture
        .take()
        .expect("the image was answered")
        .expect("a frame, not a failure");
    assert_eq!((image.size.w, image.size.h), (24, 24));
    let rgba = image.to_rgba8().expect("rgba");
    assert!(
        rgba.chunks(4).any(|px| px[3] == 255),
        "the arrow has an opaque body"
    );
    assert!(
        rgba.chunks(4).any(|px| px[3] == 0),
        "and a clear corner around it"
    );

    capture.request(BufferAsk::Announced).expect("a frame");
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    settle(&mut fixture, &mut capture);
    assert!(
        capture.take().is_none(),
        "the same cursor is not sent twice"
    );
}
