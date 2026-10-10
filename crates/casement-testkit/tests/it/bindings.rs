//! Bindings that call sill or start programs (P1-I1), through the fixture's fakes.

use casement_core::DeviceId;
use casement_core::input::{
    Argv, Binding, BindingAction, Bindings, Chord, DbusCall, DeviceClass, Edge, KeyCode,
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
    input.key(device, KEY_LEFTMETA, Edge::Press);
    input.key(device, KEY_SPACE, Edge::Press);
    input.key(device, KEY_SPACE, Edge::Release);
    input.key(device, KEY_LEFTMETA, Edge::Release);
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
