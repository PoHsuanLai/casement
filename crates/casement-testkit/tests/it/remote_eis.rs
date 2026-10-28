//! Remote input (P2-T3): the portal's unit may have an EIS socket and nobody else may, and
//! what a remote session sends enters the model tagged as remote, so it never fires a binding.

use casement_core::DeviceId;
use casement_core::input::{
    Binding, BindingAction, Bindings, Chord, DbusCall, InputEvent, InputKind, KeyCode, Origin,
    PortalSessionId,
};
use casement_ipc::dbus::ConnectRefusal;
use casement_protocols::vocab::PressEdge;
use casement_testkit::fakes::callers;
use casement_testkit::{Fixture, FixtureSpec};
use std::io::Read;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;

const KEY_LEFTMETA: KeyCode = KeyCode(125);
const KEY_SPACE: KeyCode = KeyCode(57);

/// Only the portal is handed a remote socket; the shell and another desktop program are refused
/// with the reason the D-Bus caller is told. (That the caller is who it says is the bus thread's
/// proof, tested in `casement-ipc`; a caller nobody could prove never reaches the loop.)
#[test]
fn only_the_portal_is_handed_a_remote_socket() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    assert!(fixture.connect_remote_eis(&callers::portal(), 1, 7).is_ok());
    // (case, the caller, what comes back)
    let refused = [
        (
            "the shell",
            callers::shell(),
            ConnectRefusal::RoleNotAllowed,
        ),
        (
            "another desktop program",
            callers::companion(),
            ConnectRefusal::RoleNotAllowed,
        ),
    ];
    for (case, caller, why) in refused {
        assert_eq!(
            fixture.connect_remote_eis(&caller, 1, 7).err(),
            Some(why),
            "{case}"
        );
    }
}

/// A remote session pressing a bound chord does not fire the binding.
#[test]
fn remote_keys_never_fire_a_binding() {
    let mut spec = FixtureSpec::desktop();
    spec.settings.bindings = Bindings(vec![Binding {
        chord: Chord::parse("cmd+space").expect("a chord"),
        action: BindingAction::Call(DbusCall {
            service: "org.quire.Sill".to_owned(),
            path: "/org/quire/Sill".to_owned(),
            interface: "org.quire.Sill1".to_owned(),
            method: "ToggleLauncher".to_owned(),
            args: Vec::new(),
        }),
    }]);
    let mut fixture = Fixture::start(spec).expect("fixture");
    let session = PortalSessionId(5);
    let device = DeviceId(0x8000_0014);
    for (code, edge) in [
        (KEY_LEFTMETA, PressEdge::Press),
        (KEY_SPACE, PressEdge::Press),
        (KEY_SPACE, PressEdge::Release),
        (KEY_LEFTMETA, PressEdge::Release),
    ] {
        let at = fixture.now();
        fixture.deliver_remote(InputEvent {
            origin: Origin::Remote(session),
            device,
            at,
            kind: InputKind::Key { code, edge },
        });
    }
    assert!(fixture.shell_calls().calls().is_empty());
}

/// Whether the compositor has closed its end of the socket whose client end is `fd`: reading it
/// reaches the end of the stream (anything the compositor had already written is drained first).
fn closed_by_the_compositor(fd: OwnedFd) -> bool {
    let mut end = UnixStream::from(fd);
    end.set_nonblocking(true).expect("a non-blocking end");
    let mut buffer = [0_u8; 256];
    for _ in 0..64 {
        match end.read(&mut buffer) {
            Ok(0) => return true,
            Ok(_) => {}
            Err(_) => return false,
        }
    }
    false
}

/// PORTAL-05: `EndEis` closes the compositor's end of every socket handed out for the session
/// and no other, only the portal may ask, and ending a session it does not know (or one
/// already ended) is not an error.
#[test]
fn ending_a_session_closes_its_sockets_and_only_the_portal_may_ask() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let (portal, shell) = (callers::portal(), callers::shell());
    let first = fixture.connect_remote_eis(&portal, 1, 7).expect("granted");
    let again = fixture.connect_remote_eis(&portal, 1, 3).expect("granted");
    let other = fixture.connect_remote_eis(&portal, 2, 7).expect("granted");

    assert_eq!(
        fixture.end_remote_eis(&shell, 1),
        Err(ConnectRefusal::RoleNotAllowed),
        "the shell may not end a session"
    );
    let first = UnixStream::from(first);
    let again = UnixStream::from(again);
    first.set_nonblocking(true).expect("non-blocking");
    again.set_nonblocking(true).expect("non-blocking");
    assert!(
        !closed_by_the_compositor(OwnedFd::from(first.try_clone().expect("a copy"))),
        "a refused ask closes nothing"
    );

    assert_eq!(fixture.end_remote_eis(&portal, 1), Ok(()));
    assert!(
        closed_by_the_compositor(OwnedFd::from(first)),
        "the session's first socket"
    );
    assert!(
        closed_by_the_compositor(OwnedFd::from(again)),
        "the session's second socket"
    );
    assert!(
        !closed_by_the_compositor(other),
        "another session's socket stays open"
    );
    assert_eq!(fixture.end_remote_eis(&portal, 1), Ok(()), "ending twice");
    assert_eq!(
        fixture.end_remote_eis(&portal, 77),
        Ok(()),
        "an unknown session"
    );
}

/// NEW-PI-8: the sockets a portal connection asked for go with that connection. A restarted
/// portal (a new connection) does not strand the old one's sockets, and the new one's are left
/// alone.
#[test]
fn a_portal_that_leaves_the_bus_takes_its_remote_sockets_with_it() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let portal = callers::portal();
    let first = fixture
        .connect_remote_eis_from(&portal, ":1.7", 1, 7)
        .expect("granted");
    let second = fixture
        .connect_remote_eis_from(&portal, ":1.7", 2, 7)
        .expect("granted");
    let restarted = fixture
        .connect_remote_eis_from(&portal, ":1.8", 3, 7)
        .expect("granted");
    let unowned = fixture.connect_remote_eis(&portal, 4, 7).expect("granted");

    fixture.portal_left(&portal, ":1.7");
    assert!(closed_by_the_compositor(first), "the old portal's first");
    assert!(closed_by_the_compositor(second), "the old portal's second");
    assert!(
        !closed_by_the_compositor(restarted),
        "the restarted portal's socket stays"
    );
    assert!(
        !closed_by_the_compositor(unowned),
        "a socket nobody owns is not ended by a name"
    );
    fixture.portal_left(&portal, ":1.7");
}
