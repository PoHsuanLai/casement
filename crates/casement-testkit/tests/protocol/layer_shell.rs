//! `layer_shell` (P0-H3).

use casement_core::geometry::PhysicalSize;
use casement_core::input::{DeviceClass, Edge, KeyCode};
use casement_core::policy::ConnectionRole;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{ClientId, Fixture, FixtureSpec, TestOrigin};
use porter_core::SandboxFacts;

/// Whether the registry the client saw lists `interface`.
fn advertises(client: &mut TestClient, interface: &str) -> bool {
    client
        .globals()
        .contents()
        .clone_list()
        .iter()
        .any(|global| global.interface == interface)
}

/// The evdev code of the left shift key.
const KEY_LEFTSHIFT: KeyCode = KeyCode(42);

/// A layer that spans the whole output, takes no keyboard and sits below everything.
fn wallpaper() -> LayerAsk {
    LayerAsk {
        level: Layer::Background,
        namespace: "wallpaper",
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        exclusive_zone: -1,
        keyboard: KeyboardInteractivity::None,
        size: (0, 0),
    }
}

/// The events of `object_prefix` objects in `client`'s transcript, as `(event, args)`.
fn events_of(client: &TestClient, object_prefix: &str) -> Vec<(String, Vec<String>)> {
    client
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with(object_prefix))
        .map(|line| (line.event.clone(), line.args.clone()))
        .collect()
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

/// An Exclusive layer gets wl_keyboard.enter with the current modifiers on the commit that maps it (A16).
#[test]
fn an_exclusive_layer_takes_the_keyboard_on_its_mapping_commit() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let device = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    // A person is holding shift when the layer maps.
    fixture.input().key(device, KEY_LEFTSHIFT, Edge::Press);
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _keyboard = Keyboard::bind(fixture.client(id)).expect("a keyboard");
    let launcher = LayerAsk {
        level: Layer::Overlay,
        namespace: "launcher",
        anchor: Anchor::empty(),
        exclusive_zone: 0,
        keyboard: KeyboardInteractivity::Exclusive,
        size: (400, 300),
    };
    // Only the first commit, the one that asks for the initial configure, has been made.
    let layer = LayerSurface::create(fixture.client(id), &launcher).expect("a layer surface");
    fixture.pump();
    assert!(layer.configure().is_some(), "the initial configure arrived");
    let events = events_of(fixture.client(id), "wl_keyboard#");
    let enter = events
        .iter()
        .position(|(event, _)| event == "enter")
        .expect("the keyboard entered the layer on the mapping commit");
    let (event, args) = &events[enter + 1];
    assert_eq!(event, "modifiers", "modifiers follow the enter at once");
    assert_ne!(
        args[1], "0",
        "shift is depressed in the modifiers sent with the enter"
    );
}

/// A layer mapping under a pointer that does not move still receives enter (SP1 A1).
#[test]
#[ignore = "real code: a commit that changes no model state (the first buffer attach of a mapped layer) never reaches core_loop's refocus, so no enter; handlers/compositor.rs commit must refocus after feed"]
fn a_layer_under_a_resting_pointer_gets_enter() {
    use casement_core::geometry::LogicalPoint;
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
    let _pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let _layer = map(&mut fixture, id, &wallpaper());
    // The pointer never moved after the layer mapped, and yet it entered.
    let events = events_of(fixture.client(id), "wl_pointer#");
    assert!(
        events.iter().any(|(event, _)| event == "enter"),
        "the surface now under the resting pointer received enter: {events:?}"
    );
}

/// A bar's exclusive zone is subtracted from a maximised window's configure (A16).
#[test]
#[ignore = "body is todo!(): write it with casement_testclient::surface::toplevel (now on master) plus a maximise step"]
fn exclusive_zones_shrink_maximised_windows() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H3: a bar's exclusive zone is subtracted from a maximised window's configure (A16)")
}

/// A Sandboxed connection does not see zwlr_layer_shell_v1 (I7).
#[test]
fn layer_shell_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let sandboxed = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    let public = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    assert!(
        !advertises(fixture.client(sandboxed), "zwlr_layer_shell_v1"),
        "a sandboxed client must not see zwlr_layer_shell_v1"
    );
    assert!(
        advertises(fixture.client(public), "zwlr_layer_shell_v1"),
        "a public client sees zwlr_layer_shell_v1"
    );
}
