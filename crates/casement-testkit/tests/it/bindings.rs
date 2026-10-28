//! Bindings that call sill or start programs (P1-I1), through the fixture's fakes.

use casement_core::DeviceId;
use casement_core::input::{
    Argv, Binding, BindingAction, Bindings, Chord, DbusCall, DeviceClass, KeyCode,
};
use casement_core::policy::ConnectionRole;
use casement_protocols::vocab::PressEdge;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testkit::{Fixture, FixtureSpec, TestOrigin};

const KEY_LEFTMETA: KeyCode = KeyCode(125);
const KEY_SPACE: KeyCode = KeyCode(57);

fn toggle_launcher() -> DbusCall {
    DbusCall {
        service: "org.quire.Sill".to_owned(),
        path: "/org/quire/Sill".to_owned(),
        interface: "org.quire.Sill1".to_owned(),
        method: "ToggleLauncher".to_owned(),
        args: Vec::new(),
    }
}

/// A fixture whose only binding is cmd+space doing `action`.
fn bound_to(action: BindingAction) -> Fixture {
    let mut spec = FixtureSpec::desktop();
    spec.settings.bindings = Bindings(vec![Binding {
        chord: Chord::parse("cmd+space").expect("a chord"),
        action,
    }]);
    Fixture::start(spec).expect("fixture")
}

/// Presses and releases cmd+space on `device`.
fn press_cmd_space(fixture: &mut Fixture, device: DeviceId) {
    let mut input = fixture.input();
    input.key(device, KEY_LEFTMETA, PressEdge::Press);
    input.key(device, KEY_SPACE, PressEdge::Press);
    input.key(device, KEY_SPACE, PressEdge::Release);
    input.key(device, KEY_LEFTMETA, PressEdge::Release);
}

/// A chord bound to a D-Bus call makes that call through `ShellCalls` within the frame the key
/// was pressed in.
#[test]
fn a_binding_calls_a_sill_method_within_one_frame() {
    let mut fixture = bound_to(BindingAction::Call(toggle_launcher()));
    assert!(fixture.shell_calls().calls().is_empty());
    let keyboard = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    press_cmd_space(&mut fixture, keyboard);
    assert_eq!(fixture.shell_calls().calls(), vec![toggle_launcher()]);
    assert!(fixture.spawner().started().is_empty());
}

/// A chord bound to a program starts it through the `Spawner`.
#[test]
fn a_binding_spawns_a_program() {
    let argv = Argv(vec!["foot".to_owned()]);
    let mut fixture = bound_to(BindingAction::Spawn(argv.clone()));
    assert!(fixture.spawner().started().is_empty());
    let keyboard = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    press_cmd_space(&mut fixture, keyboard);
    let started = fixture.spawner().started();
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].argv, argv);
    assert!(fixture.shell_calls().calls().is_empty());
}

/// An agent or remote origin never fires a binding.
#[test]
fn only_a_person_fires_a_binding() {
    let mut fixture = bound_to(BindingAction::Call(toggle_launcher()));
    assert!(fixture.shell_calls().calls().is_empty());
    let remote = fixture
        .input()
        .plug(DeviceClass::Keyboard, TestOrigin::Remote);
    press_cmd_space(&mut fixture, remote);
    assert!(fixture.shell_calls().calls().is_empty());
}

/// A bound chord fires over everything, as the Mac's does: while a layer surface (sill's
/// switcher) holds the keyboard exclusively the binding is still carried out and its key is not
/// delivered, but the chord's modifier is, and so is its release, because the switcher reads the
/// release of the modifier to know the person let go.
#[test]
fn a_chord_fires_while_an_exclusive_layer_holds_the_keyboard_and_it_hears_the_modifier_release() {
    let mut fixture = bound_to(BindingAction::Call(toggle_launcher()));
    let keyboard = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    // The switcher holds the keyboard exclusively as the shell's own surface (CASC-02).
    let id = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    let _keys = Keyboard::bind(fixture.client(id)).expect("a keyboard");
    let _switcher = LayerSurface::create(
        fixture.client(id),
        &LayerAsk {
            level: Layer::Overlay,
            namespace: "switcher",
            anchor: Anchor::empty(),
            exclusive_zone: 0,
            keyboard: KeyboardInteractivity::Exclusive,
            size: (400, 300),
        },
    )
    .expect("layer");
    fixture.pump();
    press_cmd_space(&mut fixture, keyboard);
    fixture.pump();
    assert_eq!(
        fixture.shell_calls().calls(),
        vec![toggle_launcher()],
        "the chord fired over the exclusive layer"
    );
    let delivered: Vec<(String, String)> = fixture
        .client(id)
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with("wl_keyboard#") && line.event == "key")
        .map(|line| (line.args[2].clone(), line.args[3].clone()))
        .collect();
    // Only the modifier reaches the layer, pressed and then released (the two differ in the
    // state argument); the space the binding took, and its release, are not delivered.
    assert_eq!(delivered.len(), 2, "{delivered:?}");
    assert_eq!(delivered[0].0, "125");
    assert_eq!(delivered[1].0, "125");
    assert_ne!(delivered[0].1, delivered[1].1);
}
