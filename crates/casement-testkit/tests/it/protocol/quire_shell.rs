//! `quire_shell` (P2-Z; filled by P2-Q2): the shell's surface roles, minimise targets and the
//! events only the compositor sees. Shell connections only. Written and ignored: the visibility
//! table needs only the global; the others wait for `casement_testclient::quire::shell`.

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

const INTERFACE: &str = "quire_shell_v1";

/// Whether the client `id` is offered `interface`.
fn sees(fixture: &mut Fixture, id: casement_testkit::ClientId, interface: &str) -> bool {
    fixture
        .client(id)
        .globals()
        .contents()
        .with_list(|list| list.iter().any(|global| global.interface == interface))
}

/// Only the shell's connection is offered `quire_shell_v1` (I7): not the agent tier, not the
/// public socket, not a sandbox.
#[test]
#[ignore = "P2-Q2: the global is created when the lane flips quire::shell::GLOBAL"]
fn only_the_shell_sees_quire_shell() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let public = fixture.connect(ConnectionRole::Public);
    let shell = fixture.connect(ConnectionRole::Shell);
    let agent = fixture.connect(ConnectionRole::Agent);
    let sandboxed = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    fixture.pump();
    assert!(sees(&mut fixture, shell, INTERFACE), "the shell");
    assert!(!sees(&mut fixture, agent, INTERFACE), "the agent tier");
    assert!(!sees(&mut fixture, public, INTERFACE), "the public socket");
    assert!(!sees(&mut fixture, sandboxed, INTERFACE), "a sandbox");
}

/// A role goes to a layer surface of the client and to one surface at a time.
#[test]
#[ignore = "P2-Q2: needs casement_testclient::quire::shell::ShellHandle"]
fn a_role_is_given_to_one_layer_surface() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _shell = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    todo!(
        "P2-Q2: set_role(bar) on a layer surface is carried into the model \
         (layers().role_holder); a second surface asking for bar raises role_taken; a plain \
         wl_surface raises bad_surface; the role is free again once the surface unmaps"
    );
}

/// A minimise target set by the dock decides where a window minimises to.
#[test]
#[ignore = "P2-Q2: needs casement_testclient::quire::shell::ShellHandle"]
fn the_dock_decides_where_a_window_minimises_to() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _shell = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    todo!(
        "P2-Q2: with a dock layer surface and a mapped window, set_minimise_target(dock, \
         identifier, x, y, w, h) stores the rectangle in global logical pixels \
         (model.minimise_target); an unknown identifier raises unknown_toplevel; \
         clear_minimise_target forgets it; the minimise travels to the rectangle"
    );
}

/// The real modifier keys reach the shell's role surfaces, including when none of them has
/// keyboard focus (hot corners).
#[test]
#[ignore = "P2-Q2: needs casement_testclient::quire::shell::ShellHandle"]
fn modifiers_reach_the_shell_without_focus() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _shell = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    todo!(
        "P2-Q2: press and release Command through the fixture's keyboard; the surface that \
         holds a role is sent modifiers(surface, mods) each time the set changes, with \
         MacMods::COMMAND then NONE, while a toplevel holds keyboard focus"
    );
}
