//! Globals a sandboxed client never sees (I7, `HIDDEN_FROM_SANDBOXES` in casement-core's
//! `tests/input.rs`): one fixture, one public and one sandboxed client, a row per interface.
//! The public client must see each, so a row cannot pass by the interface not existing.

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

/// The interface of each hidden global.
const HIDDEN: &[&str] = &[
    "ext_data_control_manager_v1",
    "zwlr_data_control_manager_v1",
    "ext_foreign_toplevel_list_v1",
    "zwp_input_method_manager_v2",
    "zwp_virtual_keyboard_manager_v1",
    "wp_security_context_manager_v1",
    "zwlr_layer_shell_v1",
    "ext_session_lock_manager_v1",
    "zwlr_output_manager_v1",
    "zwlr_output_power_manager_v1",
    "zwlr_gamma_control_manager_v1",
];

#[test]
fn hidden_globals_are_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let sandboxed = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    let public = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut sees = |client, interface: &str| {
        fixture
            .client(client)
            .globals()
            .contents()
            .with_list(|list| list.iter().any(|global| global.interface == interface))
    };
    for interface in HIDDEN {
        assert!(sees(public, interface), "a public client sees {interface}");
        assert!(
            !sees(sandboxed, interface),
            "a sandboxed client must not see {interface}"
        );
    }
}
