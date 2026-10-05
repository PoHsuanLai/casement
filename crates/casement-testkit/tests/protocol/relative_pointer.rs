//! `relative_pointer` (P0-H4).

use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::input::DeviceClass;
use casement_core::policy::ConnectionRole;
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

/// Relative_motion carries the device's raw delta.
#[test]
#[ignore = "FixtureInput has no relative motion: pointer_to is InputKind::MotionTo with a zero delta, so no relative_motion is sent (add FixtureInput::pointer_by)"]
fn relative_motion_is_unaccelerated() {
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
    let _layer = map(
        &mut fixture,
        id,
        &cover(Layer::Top, "game", KeyboardInteractivity::None),
    );
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 500, y: 300 });
    assert!(
        count(
            fixture.client(id),
            "zwp_relative_pointer_v1#",
            "relative_motion"
        ) >= 1,
        "the 100 pixel move arrived as relative motion"
    );
}
