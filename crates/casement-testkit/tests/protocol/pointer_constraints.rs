//! `pointer_constraints` (P0-H4).

use casement_core::geometry::{LogicalPoint, PhysicalSize};
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
#[ignore = "real code: the lock activates only on enter, which waits on the commit refocus fix (see layer_shell); then needs FixtureInput::pointer_by for the relative half"]
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
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 600, y: 300 });
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
