//! `security_context` (P0-H8).

use casement_core::policy::ConnectionRole;
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

/// A client of a security-context listener connects as ConnectionRole::Sandboxed with the context's facts (H8 exit).
#[test]
#[ignore = "P0-H8"]
fn a_listener_tags_its_clients_sandboxed() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    let _ = fixture.client(client);
    todo!(
        "P0-H8: a client of a security-context listener connects as ConnectionRole::Sandboxed with the context's facts (H8 exit)"
    )
}
