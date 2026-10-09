//! `pointer_constraints` (P0-H4).

use casement_core::geometry::{LogicalPoint, PhysicalSize, SubPoint, SubPx};
use casement_core::input::DeviceClass;
use casement_core::policy::ConnectionRole;
use casement_testclient::seat::constraints::Constraint;
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::seat::relative::Relative;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{ClientId, Fixture, FixtureSpec, TestOrigin};

/// A layer that spans the whole output at `level` with this keyboard interactivity.
fn cover(level: Layer, namespace: &'static str, keyboard: KeyboardInteractivity) -> LayerAsk {
    LayerAsk {
        level,
        namespace,
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        exclusive_zone: -1,
        keyboard,
        size: (0, 0),
    }
}

/// Maps `ask` the way a shell does: create, wait for the configure, acknowledge it and attach
/// a buffer of the configured size.
fn map(fixture: &mut Fixture, id: ClientId, ask: &LayerAsk) -> LayerSurface {
    let layer = LayerSurface::create(fixture.client(id), ask).expect("a layer surface");
    fixture.pump();
    let configure = layer.configure().expect("the initial configure");
    assert!(layer.ack(fixture.client(id)));
    let size = PhysicalSize {
        w: configure.width.max(ask.size.0),
        h: configure.height.max(ask.size.1),
    };
    let buffer = shm_buffer(fixture.client(id), size, Pattern::Solid(0xff20_2020));
    layer.show(fixture.client(id), &buffer);
    fixture.pump();
    layer
}

/// How many events named `event` of objects starting with `prefix` the client received.
fn count(client: &TestClient, prefix: &str, event: &str) -> usize {
    client
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with(prefix) && line.event == event)
        .count()
}

/// A locked pointer stays put and relative motion still arrives.
#[test]
fn a_lock_holds_while_focused() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 400, y: 300 });
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let _relative = Relative::bind(fixture.client(id), &pointer).expect("relative pointer");
    let layer = map(
        &mut fixture,
        id,
        &cover(Layer::Top, "game", KeyboardInteractivity::None),
    );
    let _lock = Constraint::lock(fixture.client(id), layer.surface(), &pointer).expect("lock");
    fixture.pump();
    assert_eq!(
        count(fixture.client(id), "zwp_locked_pointer_v1#", "locked"),
        1
    );
    let motions = count(fixture.client(id), "wl_pointer#", "motion");
    let relatives = count(
        fixture.client(id),
        "zwp_relative_pointer_v1#",
        "relative_motion",
    );
    fixture.input().pointer_by(
        mouse,
        SubPoint {
            x: SubPx(200 * 120),
            y: SubPx(0),
        },
    );
    assert_eq!(
        count(fixture.client(id), "wl_pointer#", "motion"),
        motions,
        "a locked pointer does not move"
    );
    assert!(
        count(
            fixture.client(id),
            "zwp_relative_pointer_v1#",
            "relative_motion"
        ) > relatives,
        "relative motion still arrives"
    );
}

/// The last `wl_pointer.motion` position the client was given, as it was written in the
/// transcript.
fn last_motion(client: &TestClient) -> Option<Vec<String>> {
    client
        .transcript()
        .0
        .iter()
        .rev()
        .find(|line| line.object.starts_with("wl_pointer#") && line.event == "motion")
        .map(|line| line.args.clone())
}

/// A confined pointer that is pushed past the surface's edge stops on its last pixel; it does not
/// stay where it was, and it is not taken off the surface (which would end the confinement).
#[test]
fn a_confined_pointer_stops_at_the_edge() {
    use super::xdg_shell::{map_toplevel, rect_of, settle, windows};
    use casement_testclient::surface::toplevel::WmBase;
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let base = WmBase::bind(fixture.client(id), 7);
    let window = map_toplevel(&mut fixture, id, &base, (200, 150));
    settle(&mut fixture);
    let a = windows(&fixture)[0];
    fixture.activate_window(a);
    let rect = rect_of(&fixture, a);
    fixture.input().pointer_to(mouse, rect.centre());
    fixture.pump();
    let _confine =
        Constraint::confine(fixture.client(id), &window.surface, &pointer).expect("confine");
    fixture.pump();
    assert_eq!(
        count(fixture.client(id), "zwp_confined_pointer_v1#", "confined"),
        1
    );
    // Far past the right and bottom edges in one motion.
    fixture.input().pointer_by(
        mouse,
        SubPoint {
            x: SubPx(10_000 * 120),
            y: SubPx(10_000 * 120),
        },
    );
    fixture.pump();
    let last = last_motion(fixture.client(id)).expect("the pointer moved");
    assert!(
        last[1].starts_with("199") && last[2].starts_with("149"),
        "the pointer stopped on the window's last pixel: {last:?}"
    );
    assert_eq!(
        count(fixture.client(id), "zwp_confined_pointer_v1#", "unconfined"),
        0,
        "and is still confined"
    );
}

/// A constraint holds while its surface has the keyboard: when another window is activated the
/// lock is released, and it holds again when the window has the keyboard back (a persistent lock).
#[test]
fn a_lock_is_released_when_another_window_takes_the_keyboard() {
    use super::xdg_shell::{map_toplevel, windows};
    use casement_testclient::surface::toplevel::WmBase;
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let base = WmBase::bind(fixture.client(id), 7);
    let first = map_toplevel(&mut fixture, id, &base, (200, 150));
    super::xdg_shell::settle(&mut fixture);
    let a = windows(&fixture)[0];
    // Near the window's top-left corner: the second window, which cascades down and right of
    // it, does not cover the pointer, so the pointer stays on the first throughout.
    let corner = super::xdg_shell::rect_of(&fixture, a).origin;
    fixture.activate_window(a);
    fixture.input().pointer_to(
        mouse,
        LogicalPoint {
            x: corner.x + 3,
            y: corner.y + 3,
        },
    );
    fixture.pump();
    let _lock = Constraint::lock(fixture.client(id), &first.surface, &pointer).expect("lock");
    fixture.pump();
    assert_eq!(
        count(fixture.client(id), "zwp_locked_pointer_v1#", "locked"),
        1,
        "the lock took hold on the window with the keyboard"
    );
    // A second window, put out of the pointer's way, takes the keyboard.
    let _second = map_toplevel(&mut fixture, id, &base, (200, 150));
    let b = *windows(&fixture).last().expect("a second window");
    fixture.place_window(b, LogicalPoint { x: 1500, y: 800 });
    fixture.advance(std::time::Duration::from_secs(1));
    fixture.activate_window(b);
    fixture.pump();
    assert_eq!(
        count(fixture.client(id), "zwp_locked_pointer_v1#", "unlocked"),
        1,
        "the first window lost the keyboard, so its lock was released"
    );
    fixture.activate_window(a);
    fixture.pump();
    assert_eq!(
        count(fixture.client(id), "zwp_locked_pointer_v1#", "locked"),
        2,
        "and it holds again when the window has the keyboard back"
    );
}
