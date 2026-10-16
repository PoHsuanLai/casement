//! `shortcuts_inhibit` (P0-H4).

use casement_core::geometry::PhysicalSize;
use casement_core::policy::ConnectionRole;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::seat::shortcuts_inhibit::Inhibitor;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{ClientId, Fixture, FixtureSpec};

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

/// With an active inhibitor a binding chord reaches the client instead of firing.
#[test]
fn an_active_inhibitor_receives_binding_chords() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    // The surface holds the keyboard exclusively only as the shell's (CASC-02).
    let id = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    let keyboard = Keyboard::bind(fixture.client(id)).expect("a keyboard");
    let layer = map(
        &mut fixture,
        id,
        &cover(Layer::Overlay, "vm", KeyboardInteractivity::Exclusive),
    );
    // The surface holds the keyboard, so its inhibitor is active at once: the compositor's
    // bindings stand aside and every chord reaches the client.
    let _inhibitor = Inhibitor::create(fixture.client(id), layer.surface(), keyboard.seat())
        .expect("an inhibitor");
    fixture.pump();
    assert_eq!(
        count(
            fixture.client(id),
            "zwp_keyboard_shortcuts_inhibitor_v1#",
            "active"
        ),
        1
    );
}
