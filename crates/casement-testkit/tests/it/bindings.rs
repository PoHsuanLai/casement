//! Bindings that call sill or start programs (P1-I1), through the fixture's fakes.

use casement_testkit::{Fixture, FixtureSpec};

/// A chord bound to a D-Bus call makes that call through `ShellCalls` within the frame the key
/// was pressed in.
#[test]
#[ignore = "P1-I1"]
fn a_binding_calls_a_sill_method_within_one_frame() {
    let fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    assert!(fixture.shell_calls().calls().is_empty());
    todo!("P1-I1: bind cmd+space to a call, press it, expect the FakeShellCalls record");
}

/// A chord bound to a program starts it through the `Spawner`.
#[test]
#[ignore = "P1-I1"]
fn a_binding_spawns_a_program() {
    let fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    assert!(fixture.spawner().started().is_empty());
    todo!("P1-I1: bind a chord to `spawn foot`, press it, expect the FakeSpawner record");
}

/// An agent or remote origin never fires a binding.
#[test]
#[ignore = "P1-I1"]
fn only_a_person_fires_a_binding() {
    let fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    assert!(fixture.shell_calls().calls().is_empty());
    todo!("P1-I1: the same chord from Origin::Remote makes no call");
}
