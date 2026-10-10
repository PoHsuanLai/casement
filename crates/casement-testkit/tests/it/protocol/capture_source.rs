//! `capture_source` (P2-Q3): toplevel image capture. A client makes a source from a foreign
//! toplevel handle and captures the window through the same copy session output capture uses.
//! The client side is `casement_testclient::capture::toplevel_source::WindowCapture`.

use crate::protocol::xdg_shell::{boot, map_toplevel, windows};
use casement_core::geometry::{LogicalSize, PhysicalSize};
use casement_core::policy::ConnectionRole;
use casement_core::{OutputName, WindowId};
use casement_testclient::capture::toplevel_source::{CaptureFailure, CursorMode, WindowCapture};
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testclient::{Pattern, Pumped, shm_buffer};
use casement_testkit::{ClientId, Fixture, FixtureSpec, FrameCount};
use casement_wayland::capture_source::capture_size;
use casement_wayland::foreign_toplevel_list::identifier_of;
use ds_core::geometry::scale::Scale;
use porter_core::SandboxFacts;

const INTERFACE: &str = "ext_foreign_toplevel_image_capture_source_manager_v1";

/// A public client is offered the manager, a sandboxed one is not (I7, like output capture).
#[test]
fn a_public_client_is_offered_window_capture_and_a_sandbox_is_not() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let public = fixture.connect(ConnectionRole::Public);
    let sandboxed = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    fixture.pump();
    let mut sees = |id| {
        fixture
            .client(id)
            .globals()
            .contents()
            .with_list(|list| list.iter().any(|global| global.interface == INTERFACE))
    };
    assert!(sees(public), "the public socket");
    assert!(!sees(sandboxed), "a sandbox");
}

/// A 4K output at `scale` (120ths), so a window of any size here fits its logical area.
fn big(scale: u32) -> FixtureSpec {
    let output = format!("HEADLESS-1:3840x2160@60000:{scale}:0,0");
    FixtureSpec::desktop().with_outputs(&[output.as_str()])
}

fn output() -> OutputName {
    OutputName::parse("HEADLESS-1").expect("a name")
}

/// Pumps the compositor and the capture until neither moves.
fn settle(fixture: &mut Fixture, capture: &mut WindowCapture) {
    for _ in 0..100 {
        let compositor = fixture.pump();
        let client = capture.pump().expect("pump the capture");
        if (compositor, client) == (Pumped::Quiet, Pumped::Quiet) {
            return;
        }
    }
    panic!("the compositor and the capture did not go quiet");
}

/// A mapped toplevel of one colour.
fn map_solid(
    fixture: &mut Fixture,
    id: ClientId,
    base: &WmBase,
    size: (u32, u32),
    argb: u32,
) -> Toplevel {
    let top = base.toplevel(fixture.client(id));
    top.commit_bare();
    fixture.pump();
    assert!(top.ack_latest().is_some(), "the initial configure arrives");
    show_solid(fixture, id, &top, size, argb);
    top
}

/// Commits a buffer of one colour on `top`.
fn show_solid(fixture: &mut Fixture, id: ClientId, top: &Toplevel, size: (u32, u32), argb: u32) {
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize {
            w: size.0,
            h: size.1,
        },
        Pattern::Solid(argb),
    );
    top.show(&buffer);
    fixture.pump();
}

/// A capture of the first window, opened and settled (its constraints are in).
fn opened(fixture: &mut Fixture, id: ClientId, window: WindowId) -> WindowCapture {
    let mut capture = WindowCapture::start(fixture.client(id).globals()).expect("the managers");
    settle(fixture, &mut capture);
    capture
        .open(&identifier_of(window), CursorMode::Hidden)
        .expect("a session on the window");
    settle(fixture, &mut capture);
    capture
}

/// The buffer a session announces is `round(logical × scale)` of the window's content at its
/// output's scale (1.25, 1.5 and 2), and again after the scale changes.
#[test]
fn a_window_capture_is_sized_at_the_output_scale() {
    for scale in [150_u32, 180, 240] {
        let mut fixture = Fixture::start(big(scale)).expect("fixture");
        let id = fixture.connect(ConnectionRole::Public);
        fixture.pump();
        let base = WmBase::bind(fixture.client(id), 7);
        let _top = map_toplevel(&mut fixture, id, &base, (801, 601));
        let window = windows(&fixture)[0];
        let mut capture = opened(&mut fixture, id, window);
        let want = capture_size(LogicalSize { w: 801, h: 601 }, Scale(scale));
        assert_eq!(capture.size(), Some((want.w, want.h)), "at {scale}/120");
        // The output's scale changes: the next frame fails, the session says the new size, and
        // a buffer of that size is drawn.
        fixture.rescale(&output(), Scale(120));
        capture.request().expect("a request");
        settle(&mut fixture, &mut capture);
        assert_eq!(
            capture.take().expect("answered").expect_err("a failure"),
            CaptureFailure::BufferConstraints,
            "the old size no longer fits"
        );
        assert_eq!(capture.size(), Some((801, 601)), "at 1x after {scale}/120");
        capture.request().expect("a request");
        settle(&mut fixture, &mut capture);
        let frame = capture
            .take()
            .expect("answered")
            .expect("a frame, not a failure");
        assert_eq!((frame.width, frame.height), (801, 601));
    }
}

/// A window that does not change costs a session no frames after the first, over 120 virtual
/// frames; a commit releases exactly one request, with the new pixels.
#[test]
fn a_static_window_costs_no_frames() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let top = map_solid(&mut fixture, id, &base, (320, 200), 0xffff_0000);
    let window = windows(&fixture)[0];
    let mut capture = opened(&mut fixture, id, window);
    capture.request().expect("a request");
    settle(&mut fixture, &mut capture);
    let first = capture.take().expect("answered").expect("the first frame");
    assert_eq!(first.rgba[..4], [255, 0, 0, 255]);

    capture.request().expect("a request");
    for _ in 0..120 {
        let _ = fixture.frames(&output(), FrameCount(1));
        capture.pump().expect("pump the capture");
    }
    assert!(capture.take().is_none(), "no ready and no failed arrived");

    show_solid(&mut fixture, id, &top, (320, 200), 0xff00_ff00);
    settle(&mut fixture, &mut capture);
    let second = capture.take().expect("released").expect("the new pixels");
    assert_eq!(second.rgba[..4], [0, 255, 0, 255]);

    capture.request().expect("a request");
    for _ in 0..120 {
        let _ = fixture.frames(&output(), FrameCount(1));
        capture.pump().expect("pump the capture");
    }
    assert!(capture.take().is_none(), "one commit released one request");
}

/// What lies over a window on the screen is not in its capture.
#[test]
fn an_overlapped_window_is_captured_whole() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let _below = map_solid(&mut fixture, id, &base, (400, 300), 0xffff_0000);
    let _above = map_solid(&mut fixture, id, &base, (400, 300), 0xff00_00ff);
    let below = windows(&fixture)[0];
    let mut capture = opened(&mut fixture, id, below);
    capture.request().expect("a request");
    settle(&mut fixture, &mut capture);
    let frame = capture.take().expect("answered").expect("a frame");
    assert!(
        frame.rgba.chunks(4).all(|pixel| pixel == [255, 0, 0, 255]),
        "every pixel is the lower window's own"
    );
}

/// A session whose window closes is stopped; a source made for a handle that is gone is stopped
/// at once.
#[test]
fn a_closed_window_ends_its_sessions() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let top = map_toplevel(&mut fixture, id, &base, (320, 200));
    let window = windows(&fixture)[0];
    let mut capture = opened(&mut fixture, id, window);
    assert!(!capture.is_stopped());
    // A second client of the list that has seen the window, before it closes.
    let mut late = WindowCapture::start(fixture.client(id).globals()).expect("the managers");
    settle(&mut fixture, &mut late);
    top.destroy();
    settle(&mut fixture, &mut capture);
    settle(&mut fixture, &mut late);
    assert!(capture.is_stopped(), "the window closed");
    late.open(&identifier_of(window), CursorMode::Hidden)
        .expect("a source from the old handle");
    settle(&mut fixture, &mut late);
    assert!(late.is_stopped(), "a handle that is gone has no window");
}
