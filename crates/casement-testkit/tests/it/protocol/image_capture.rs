//! `image_capture` (P1-C1): output capture through ext-image-copy-capture, read by
//! `casement_testclient::capture` (which folds the events with `desktop_client`'s reducer).

use casement_core::OutputName;
use casement_core::policy::ConnectionRole;
use casement_testclient::Pumped;
use casement_testclient::capture::{BufferAsk, CaptureFailure, CursorMode, OutputCapture, Size};
use casement_testkit::{ClientId, Fixture, FixtureSpec, FrameCount, ShotTarget};

/// One small output, so a frame is a few hundred kilobytes.
fn small() -> FixtureSpec {
    FixtureSpec::desktop().with_outputs(&["HEADLESS-1:640x480@60000:120:0,0"])
}

fn name(text: &str) -> OutputName {
    OutputName::parse(text).expect("a name")
}

/// Pumps the compositor and the capture until neither moves.
fn settle(fixture: &mut Fixture, capture: &mut OutputCapture) {
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
/// settled (its constraints are in).
fn opened(spec: FixtureSpec, output: &str) -> (Fixture, ClientId, OutputCapture) {
    let mut fixture = Fixture::start(spec).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut capture = OutputCapture::start(fixture.client(client).globals()).expect("the managers");
    settle(&mut fixture, &mut capture);
    capture
        .open(output, CursorMode::Hidden)
        .expect("a session on the output");
    settle(&mut fixture, &mut capture);
    (fixture, client, capture)
}

/// The captured frame equals the frame scanout shows, byte for byte on pixman.
#[test]
fn a_capture_equals_the_scanout_frame() {
    let (mut fixture, _client, mut capture) = opened(small(), "HEADLESS-1");
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
