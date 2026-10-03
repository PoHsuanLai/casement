//! `security_context` (P0-H8). Runs once the fixture and test client core land.

use casement_core::policy::ConnectionRole;
use casement_testclient::TestClient;
use casement_testclient::misc::security_context::{EngineFacts, Listener};
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SandboxFacts;

fn interfaces(client: &TestClient) -> Vec<String> {
    client
        .globals()
        .contents()
        .with_list(|list| list.iter().map(|global| global.interface.clone()).collect())
}

fn facts() -> EngineFacts {
    EngineFacts {
        engine: "org.flatpak".to_owned(),
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }
}

/// A client of a security-context listener connects as ConnectionRole::Sandboxed with the
/// context's facts (H8 exit): it sees the sandbox's globals (I7) and not the privileged ones,
/// and not the security-context manager itself.
#[test]
#[ignore = "P0-H8: written; runs once P0-B-headless (Fixture::start) and P0-T-client land"]
fn a_listener_tags_its_clients_sandboxed() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let engine = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let dir = tempfile::tempdir().expect("a private socket directory");
    let mut listener = Listener::commit(
        fixture.client(engine).globals(),
        dir.path(),
        "sandbox.sock",
        &facts(),
    )
    .expect("the listener");
    fixture.pump();
    let stream = listener.connect().expect("connect to the listener");
    let mut sandboxed = TestClient::over(stream.into()).expect("the sandboxed client");
    for _ in 0..8 {
        fixture.pump();
        sandboxed.pump();
        listener.pump().expect("pump the engine side");
    }
    let seen = interfaces(&sandboxed);
    assert!(seen.iter().any(|name| name == "wl_compositor"));
    for hidden in [
        "wp_security_context_manager_v1",
        "zwp_input_method_manager_v2",
        "zwp_virtual_keyboard_manager_v1",
        "ext_foreign_toplevel_list_v1",
        "ext_data_control_manager_v1",
    ] {
        assert!(
            !seen.iter().any(|name| name == hidden),
            "a sandboxed client must not see {hidden}"
        );
    }
    let public = interfaces(fixture.client(engine));
    assert!(
        public
            .iter()
            .any(|name| name == "wp_security_context_manager_v1")
    );
    let role = ConnectionRole::Sandboxed(SandboxFacts::Engine {
        engine: facts().engine,
        app: facts().app,
        instance: facts().instance,
    });
    assert_eq!(
        casement_core::policy::advertised(
            casement_core::policy::GlobalKind::SecurityContext,
            &role
        ),
        casement_core::policy::Advertise::No
    );
}

/// Closing the engine's pipe ends the listener: a later connection is never served.
#[test]
#[ignore = "P0-H8: written; runs once P0-B-headless (Fixture::start) and P0-T-client land"]
fn closing_the_pipe_stops_the_listener() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let engine = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let dir = tempfile::tempdir().expect("a private socket directory");
    let listener = Listener::commit(
        fixture.client(engine).globals(),
        dir.path(),
        "closed.sock",
        &facts(),
    )
    .expect("the listener");
    fixture.pump();
    let stream = listener.connect().expect("connect before the close");
    drop(stream);
    listener.close().expect("close the pipe");
    fixture.pump();
    let dir_path = dir.path().join("closed.sock");
    let late = std::os::unix::net::UnixStream::connect(&dir_path);
    let mut client = late.ok().map(|stream| TestClient::over(stream.into()));
    for _ in 0..8 {
        fixture.pump();
        if let Some(Ok(client)) = client.as_mut() {
            client.pump();
        }
    }
    assert!(
        client.is_none_or(|result| result.is_err()),
        "a closed listener serves nobody"
    );
}
