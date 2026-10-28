//! `data_device` (P0-H5).

use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::input::{ButtonCode, DeviceClass};
use casement_core::policy::ConnectionRole;
use casement_protocols::vocab::PressEdge;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::selection::data_device::DataDevice;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{ClientId, Fixture, FixtureSpec, TestOrigin};

/// The evdev code of the left mouse button.
const BTN_LEFT: ButtonCode = ButtonCode(0x110);

/// A layer of `size` at the centre of the output, at `level`, taking the keyboard exclusively.
pub(crate) fn focused(level: Layer, namespace: &'static str) -> LayerAsk {
    LayerAsk {
        level,
        namespace,
        anchor: Anchor::empty(),
        exclusive_zone: 0,
        keyboard: KeyboardInteractivity::Exclusive,
        size: (400, 300),
    }
}

/// Maps `ask` the way a shell does: create, wait for the configure, acknowledge it and attach
/// a buffer of the configured size.
pub(crate) fn map(fixture: &mut Fixture, id: ClientId, ask: &LayerAsk) -> LayerSurface {
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

/// Whether `client`'s transcript has `event` from an object whose name starts with `prefix`.
pub(crate) fn saw(client: &TestClient, prefix: &str, event: &str) -> bool {
    client
        .transcript()
        .0
        .iter()
        .any(|line| line.object.starts_with(prefix) && line.event == event)
}

/// A selection set by A is offered to B when B gains keyboard focus.
#[test]
fn a_selection_reaches_the_focused_client() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    // The shell's own surfaces take the keyboard exclusively; another client's do not (CASC-02).
    let a = fixture.connect(ConnectionRole::Shell);
    let b = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    let keyboard_a = Keyboard::bind(fixture.client(a)).expect("keyboard");
    let _keyboard_b = Keyboard::bind(fixture.client(b)).expect("keyboard");
    let devices_a = DataDevice::bind(fixture.client(a), keyboard_a.seat(), None).expect("device");
    let _devices_b = DataDevice::bind(fixture.client(b), keyboard_a.seat(), None);
    let _ = _devices_b;
    // A holds the keyboard and sets the clipboard.
    let _layer_a = map(&mut fixture, a, &focused(Layer::Top, "a"));
    devices_a.copy(fixture.client(a), "text/plain", 0);
    fixture.pump();
    assert!(!saw(fixture.client(b), "wl_data_device#", "selection"));
    // B takes the keyboard: the selection is offered to it.
    let _layer_b = map(&mut fixture, b, &focused(Layer::Overlay, "b"));
    assert!(saw(fixture.client(b), "wl_data_device#", "data_offer"));
    assert!(saw(fixture.client(b), "wl_data_device#", "selection"));
}

/// A drag from a layer surface in the middle of the output, dropped on a drop zone in the
/// corner; the fixture and the drop zone's client.
fn drop_on_a_zone() -> (Fixture, ClientId) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let source = fixture.connect(ConnectionRole::Public);
    let target = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let pointer = Pointer::bind(fixture.client(source)).expect("pointer");
    let drag = DataDevice::bind(fixture.client(source), pointer.seat(), None).expect("source");
    let target_pointer = Pointer::bind(fixture.client(target)).expect("pointer");
    let _drop = DataDevice::bind(
        fixture.client(target),
        target_pointer.seat(),
        Some("text/plain"),
    )
    .expect("target");
    // The source is a layer in the middle of the output, the target a drop zone in the corner.
    let origin = map(&mut fixture, source, &focused(Layer::Top, "source"));
    let _zone = map(
        &mut fixture,
        target,
        &LayerAsk {
            level: Layer::Overlay,
            namespace: "drop-zone",
            anchor: Anchor::Top | Anchor::Left,
            exclusive_zone: -1,
            keyboard: KeyboardInteractivity::None,
            size: (200, 200),
        },
    );
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 540 });
    fixture.input().button(mouse, BTN_LEFT, PressEdge::Press);
    let serial = pointer
        .press_serial()
        .expect("the press reached the source");
    drag.drag(
        fixture.client(source),
        origin.surface(),
        "text/plain",
        serial,
    );
    fixture.pump();
    // The pointer carries the drag over the layer in the corner and lets go there.
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 100, y: 100 });
    assert!(saw(fixture.client(target), "wl_data_device#", "enter"));
    fixture.input().button(mouse, BTN_LEFT, PressEdge::Release);
    assert!(
        saw(fixture.client(target), "wl_data_device#", "drop"),
        "the drop reached the layer surface"
    );
    (fixture, target)
}

/// A drag from a window drops on a layer surface (H5 exit).
#[test]
fn drag_and_drop_into_a_layer_surface() {
    let (mut fixture, target) = drop_on_a_zone();
    fixture.pump();
    assert!(saw(fixture.client(target), "wl_data_device#", "drop"));
}

/// wl_data_device.drop: the offer introduced at enter stays the client's to read until it
/// finishes it, so the drop is not followed by `leave` (which tells a client to destroy it).
#[test]
fn a_drop_is_not_followed_by_leave() {
    let (mut fixture, target) = drop_on_a_zone();
    fixture.pump();
    assert!(
        !saw(fixture.client(target), "wl_data_device#", "leave"),
        "the offer outlives the drop"
    );
}
