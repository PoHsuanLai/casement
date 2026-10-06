//! A client the fixture does not drive (the wlcs shim's kind): its socket is adopted, it talks
//! from its own end, and the fixture can still name its window and put it somewhere.

use casement_core::WindowId;
use casement_core::geometry::{LogicalPoint, LogicalRect, PhysicalSize};
use casement_core::input::DeviceClass;
use casement_core::policy::ConnectionRole;
use casement_layout::{LayoutKnobs, RustScene, SceneLayout, spec_of};
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::surface::toplevel::WmBase;
use casement_testclient::{Pattern, Pumped, TestClient, shm_buffer};
use casement_testkit::{ClientId, Fixture, FixtureSpec, TestOrigin};
use std::time::Duration;
use wayland_client::Proxy;

/// Pumps the fixture and the one client it does not own until neither moves.
fn settle(fixture: &mut Fixture, client: &mut TestClient) {
    for _ in 0..64 {
        let server = fixture.pump();
        if (server, client.pump()) == (Pumped::Quiet, Pumped::Quiet) {
            return;
        }
    }
    panic!("the adopted client and the fixture did not go quiet");
}

/// A mapped 200x100 toplevel of an adopted client, and the object id of its surface.
fn boot() -> (Fixture, ClientId, TestClient, u32) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let (id, fd) = fixture.adopt_socket(ConnectionRole::Public);
    let mut client = TestClient::over(fd).expect("the client connects over the adopted end");
    settle(&mut fixture, &mut client);
    let base = WmBase::bind(&mut client, 7);
    let top = base.toplevel(&mut client);
    top.commit_bare();
    settle(&mut fixture, &mut client);
    assert!(top.ack_latest().is_some(), "the initial configure arrives");
    let buffer = shm_buffer(
        &mut client,
        PhysicalSize { w: 200, h: 100 },
        Pattern::Gradient,
    );
    top.show(&buffer);
    settle(&mut fixture, &mut client);
    let object = top.wl_surface().id().protocol_id();
    (fixture, id, client, object)
}

fn rect_of(fixture: &Fixture, window: WindowId) -> LogicalRect {
    let model = fixture.model();
    RustScene
        .arrange(&spec_of(model), &LayoutKnobs::of(model.settings()))
        .windows
        .iter()
        .find(|target| target.window == window)
        .map(|target| target.rect)
        .expect("the window is arranged")
}

#[test]
fn the_toplevel_of_an_adopted_clients_surface_is_found() {
    let (fixture, id, _client, object) = boot();
    let window = fixture
        .toplevel_of(id, object)
        .expect("the surface is a toplevel");
    assert!(fixture.model().windows().contains_key(&window));
    assert_eq!(
        fixture.toplevel_of(id, object + 1000),
        None,
        "no such object"
    );
}

#[test]
fn a_placed_window_keeps_its_size_and_moves_its_corner() {
    let (mut fixture, id, mut client, object) = boot();
    let window = fixture.toplevel_of(id, object).expect("a toplevel");
    let before = rect_of(&fixture, window);
    let at = LogicalPoint { x: 41, y: 67 };
    fixture.place_window(window, at);
    settle(&mut fixture, &mut client);
    let after = rect_of(&fixture, window);
    assert_eq!(after, LogicalRect::new(at, before.size));
}

/// The surface-local position of the newest `wl_pointer.motion` the client was sent.
fn last_motion(client: &TestClient) -> Option<(f64, f64)> {
    let lines = &client.transcript().0;
    let line = lines.iter().rev().find(|line| line.event == "motion")?;
    let coord = |at: usize| line.args.get(at)?.parse::<f64>().ok();
    Some((coord(1)?, coord(2)?))
}

/// A surface that moves under a pointer that does not tells its client the new surface-local
/// position (what its client holds would be stale otherwise: wlcs
/// `ClientSurfaceEventsTest.surface_moves_while_under_pointer`).
#[test]
fn a_surface_moved_under_a_still_pointer_gets_a_motion() {
    let (mut fixture, id, mut client, object) = boot();
    let _pointer = Pointer::bind(&client).expect("a pointer");
    let window = fixture.toplevel_of(id, object).expect("a toplevel");
    let device = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    // Let any glide finish before and after each move, so the hit test sees the end.
    fixture.place_window(window, LogicalPoint { x: 100, y: 100 });
    fixture.advance(Duration::from_secs(1));
    fixture.refocus_pointer();
    fixture
        .input()
        .pointer_to(device, LogicalPoint { x: 150, y: 140 });
    settle(&mut fixture, &mut client);
    assert_eq!(
        last_motion(&client),
        None,
        "the pointer entered; nothing moved over it"
    );
    fixture.place_window(window, LogicalPoint { x: 120, y: 90 });
    fixture.advance(Duration::from_secs(1));
    fixture.refocus_pointer();
    settle(&mut fixture, &mut client);
    assert_eq!(last_motion(&client), Some((30.0, 50.0)));
}
