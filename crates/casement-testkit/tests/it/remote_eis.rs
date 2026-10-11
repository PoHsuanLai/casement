//! Remote input (P2-T3): the portal's unit may have an EIS socket and nobody else may, and
//! what a remote session sends enters the model tagged as remote, so it never fires a binding.

use casement_core::DeviceId;
use casement_core::input::{
    Binding, BindingAction, Bindings, Chord, DbusCall, Edge, InputEvent, InputKind, KeyCode,
    Origin, PortalSessionId,
};
use casement_ipc::dbus::ConnectRefusal;
use casement_testkit::fakes::FakeCgroups;
use casement_testkit::{Fixture, FixtureSpec};

const KEY_LEFTMETA: KeyCode = KeyCode(125);
const KEY_SPACE: KeyCode = KeyCode(57);

/// Only `transom.service` is handed a remote socket; the shell's unit, a browser and a process
/// nobody knows are refused with the reason the D-Bus caller is told.
#[test]
fn only_the_portals_unit_is_handed_a_remote_socket() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let cgroups = FakeCgroups::new();
    cgroups.run_in_service(200, "transom.service");
    cgroups.run_in_service(201, "sill.service");
    cgroups.run_in_service(202, "firefox.service");
    assert!(fixture.connect_remote_eis(&cgroups, 200, 1, 7).is_ok());
    // (case, the process, what comes back)
    let refused = [
        ("the shell's unit", 201, ConnectRefusal::RoleNotAllowed),
        ("a browser", 202, ConnectRefusal::RoleNotAllowed),
        ("a process nobody knows", 999, ConnectRefusal::UnknownUnit),
    ];
    for (case, pid, why) in refused {
        assert_eq!(
            fixture.connect_remote_eis(&cgroups, pid, 1, 7).err(),
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
        (KEY_LEFTMETA, Edge::Press),
        (KEY_SPACE, Edge::Press),
        (KEY_SPACE, Edge::Release),
        (KEY_LEFTMETA, Edge::Release),
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
