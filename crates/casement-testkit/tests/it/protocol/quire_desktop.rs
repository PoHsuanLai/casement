//! `quire_desktop` (P2-Z; filled by P2-Q1): the desktop model of apps, windows and Spaces, read
//! by the shell and the agent tier, managed by the shell alone. The tests are written and
//! ignored: the visibility table needs only the global to be advertised; the others wait for the
//! client helper the lane writes in `casement_testclient::quire::desktop`.

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

const INTERFACE: &str = "quire_desktop_manager_v1";

/// Whether the client `id` is offered `interface`.
fn sees(fixture: &mut Fixture, id: casement_testkit::ClientId, interface: &str) -> bool {
    fixture
        .client(id)
        .globals()
        .contents()
        .with_list(|list| list.iter().any(|global| global.interface == interface))
}

/// The shell and the agent tier are offered the manager; the public socket and a sandbox are not
/// (I7).
#[test]
#[ignore = "P2-Q1: the global is created when the lane flips quire::desktop::GLOBAL"]
fn only_the_shell_and_the_agent_tier_see_the_manager() {
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
    assert!(sees(&mut fixture, agent, INTERFACE), "the agent tier");
    assert!(!sees(&mut fixture, public, INTERFACE), "the public socket");
    assert!(!sees(&mut fixture, sandboxed, INTERFACE), "a sandbox");
}

/// Binding sends an `app`, `window` and `space` event for every existing object, each followed
/// by its state burst, then `active` and `done`; the transcript is a golden.
#[test]
#[ignore = "P2-Q1: needs casement_testclient::quire::desktop::DesktopWatch"]
fn binding_sends_the_whole_desktop() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _shell = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    todo!(
        "P2-Q1: map two toplevels of one app, bind, and assert one app (identity, proof, life, \
         visibility, mru), two windows (role, state, place, output, geometry, emphasis, \
         placement) and the Space (id equals ext_workspace's id, kind, position, activity), \
         then active and done; check the transcript golden"
    );
}

/// Every management request on an agent connection is `not_permitted`; the shell's are carried
/// out.
#[test]
#[ignore = "P2-Q1: needs casement_testclient::quire::desktop::DesktopWatch"]
fn an_agent_connection_is_refused_management() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _agent = fixture.connect(ConnectionRole::Agent);
    fixture.pump();
    todo!(
        "P2-Q1: from an agent connection send create_space, app.hide, app.quit, window.close, \
         window.minimise, window.place, window.to_space and space.remove; each raises \
         quire_desktop_manager_v1.not_permitted and the model is unchanged; reads keep working"
    );
}

/// The shell's window requests reach the model the way the window's own requests do.
#[test]
#[ignore = "P2-Q1: needs casement_testclient::quire::desktop::DesktopWatch"]
fn the_shells_requests_reach_the_model() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _shell = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    todo!(
        "P2-Q1: window.place(left) tiles the window, window.minimise and unminimise round-trip, \
         window.full_screen(on) takes a Space, app.hide and unhide change visibility, \
         space.activate switches, create_space adds a Space; each change arrives back as an event"
    );
}

/// A window that closes is told `closed`; a Space that is removed is told `removed`.
#[test]
#[ignore = "P2-Q1: needs casement_testclient::quire::desktop::DesktopWatch"]
fn the_manager_follows_what_goes_away() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _shell = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    todo!(
        "P2-Q1: unmap a toplevel and see window.closed and, for the app's last window, \
         app.closed; remove a Space and see space.removed; a manager that sent stop gets \
         finished and no new object"
    );
}
