//! `capture_source` (P2-Z; filled by P2-Q3): toplevel image capture. A client makes a source from
//! a foreign toplevel handle and captures the window through the same copy session output capture
//! uses. Written and ignored: the visibility test needs only the global; the others wait for
//! `casement_testclient::capture::toplevel_source`.

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

const INTERFACE: &str = "ext_foreign_toplevel_image_capture_source_manager_v1";

/// A public client is offered the manager, a sandboxed one is not (I7, like output capture).
#[test]
#[ignore = "P2-Q3: the global is created with smithay's ToplevelCaptureSourceState"]
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

/// The buffer a session announces is `round(logical × scale)` of the window's content at its
/// output's scale (1.25, 1.5 and 2), and again after the scale changes.
#[test]
#[ignore = "P2-Q3: needs casement_testclient::capture::toplevel_source"]
fn a_window_capture_is_sized_at_the_output_scale() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    todo!(
        "P2-Q3: for scale in [150, 180, 240] (120ths) map an 801x601 toplevel, make a source \
         from its foreign handle, open a session and assert the constraints equal \
         capture_size(801x601, scale); change the output's scale and see updated constraints"
    );
}

/// A window that does not change costs a session no frames after the first, over 120 virtual
/// frames; a change releases the held request.
#[test]
#[ignore = "P2-Q3: needs casement_testclient::capture::toplevel_source"]
fn a_static_window_costs_no_frames() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    todo!(
        "P2-Q3: capture a mapped window once, ask for a frame again and advance 120 frames: no \
         ready and no failed arrives; then the window commits and the held request is answered \
         with the new pixels in the next frame"
    );
}

/// A session whose window closes is stopped; a source made for a handle that is gone is stopped
/// at once.
#[test]
#[ignore = "P2-Q3: needs casement_testclient::capture::toplevel_source"]
fn a_closed_window_ends_its_sessions() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    todo!(
        "P2-Q3: unmap the captured toplevel and see the session stopped; a source created from a \
         handle that has already closed gets a stopped session"
    );
}
