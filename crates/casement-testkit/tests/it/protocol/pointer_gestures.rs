//! `pointer_gestures` (P0-H4).

use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::geometry::{SubPoint, SubPx};
use casement_core::input::DeviceClass;
use casement_core::input::{GestureEnd, GestureStep};
use casement_core::policy::ConnectionRole;
use casement_testclient::seat::gestures::Gestures;
use casement_testclient::seat::pointer::Pointer;
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

/// A two-finger swipe begins, updates and ends at the focused client.
#[test]
fn a_swipe_the_system_does_not_take_reaches_the_client() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let pad = fixture.input().plug(
        DeviceClass::Touchpad,
        TestOrigin::Physical(DeviceClass::Touchpad),
    );
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 400, y: 300 });
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let _gestures = Gestures::bind(fixture.client(id), &pointer).expect("gestures");
    let _layer = map(
        &mut fixture,
        id,
        &cover(Layer::Top, "swipe", KeyboardInteractivity::None),
    );
    fixture
        .input()
        .swipe(pad, GestureStep::Begin { fingers: 2 });
    fixture.input().swipe(
        pad,
        GestureStep::Update(SubPoint {
            x: SubPx(240),
            y: SubPx(0),
        }),
    );
    fixture
        .input()
        .swipe(pad, GestureStep::End(GestureEnd::Lifted));
    let client = fixture.client(id);
    for event in ["begin", "update", "end"] {
        assert_eq!(
            count(client, "zwp_pointer_gesture_swipe_v1#", event),
            1,
            "one swipe {event}"
        );
    }
}
