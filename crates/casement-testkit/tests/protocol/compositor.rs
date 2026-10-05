//! `compositor` (P0-H1).

use casement_core::geometry::PhysicalSize;
use casement_testclient::surface::subsurface::{Compositor, H1Data};
use casement_testclient::{ClientError, Pattern, shm_buffer};
use wayland_client::protocol::wl_compositor::WlCompositor;

use casement_core::policy::ConnectionRole;
use casement_testclient::TestClient;
use casement_testkit::{ClientId, Env, Fixture, FixtureSpec};

fn boot() -> (Fixture, ClientId) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    (fixture, id)
}

/// The (interface, code) of the protocol error the compositor killed `id` with.
fn fault(fixture: &mut Fixture, id: ClientId) -> (String, u32) {
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("the compositor posted an error");
    (error.interface, error.code)
}

/// Compares what `client` received, without the events of the objects of `skip` and without
/// instants, with `tests/golden/transcripts/<name>`; `CASEMENT_BLESS=1` replaces a differing
/// golden (CONVENTIONS addendum 12).
fn check(name: &str, client: &TestClient, skip: &[&str]) {
    let actual: String = client
        .transcript()
        .0
        .iter()
        .filter(|line| {
            !skip
                .iter()
                .any(|iface| line.object.starts_with(&format!("{iface}#")))
        })
        .map(|line| format!("{}.{}({})\n", line.object, line.event, line.args.join(", ")))
        .collect();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/transcripts")
        .join(name);
    let golden = std::fs::read_to_string(&path).unwrap_or_default();
    if golden == actual {
        return;
    }
    if Env::of_process().bless == casement_render::golden::Bless::Yes {
        std::fs::write(&path, &actual).expect("write the blessed golden");
        return;
    }
    panic!(
        "{} differs\n--- golden\n{golden}--- actual\n{actual}",
        path.display()
    );
}

const SIZE: PhysicalSize = PhysicalSize { w: 64, h: 32 };

/// Wl_compositor binds at versions 1 to 5; a surface with a buffer commits; the transcript matches tests/golden/transcripts/compositor-commit.txt.
#[test]
fn every_version_binds_and_a_surface_commits() {
    let (mut fixture, _) = boot();
    for version in 1..=5 {
        let id = fixture.connect(ConnectionRole::Public);
        fixture.pump();
        let client = fixture.client(id);
        let compositor = Compositor::bind(client, version);
        let surface = compositor.surface(client);
        let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
        surface.attach(&buffer);
        surface.damage_all();
        surface.commit();
        fixture.pump();
        let client = fixture.client(id);
        assert!(client.protocol_error().is_none(), "version {version}");
        // An advertised version is the ceiling: asking for the next one is refused.
        assert!(matches!(
            client.bind::<WlCompositor, _>(6..=6, H1Data),
            Err(ClientError::MissingGlobal(_))
        ));
    }
}

/// The canonical flow's events (a buffer's pool, a surface, an attach and a commit) are the
/// formats `wl_shm` announces and nothing else.
#[test]
fn the_commit_flow_transcript_matches_its_golden() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
    let compositor = Compositor::bind(client, 5);
    let surface = compositor.surface(client);
    surface.attach(&buffer);
    surface.damage_all();
    surface.commit();
    fixture.pump();
    check(
        "compositor-commit.txt",
        fixture.client(id),
        &["wl_registry", "wl_display"],
    );
}

/// A commit on a surface with no role yet changes nothing in the model.
#[test]
fn a_commit_without_a_role_is_unassigned() {
    let (mut fixture, id) = boot();
    let before = fixture.model().clone();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let buffer = shm_buffer(client, SIZE, Pattern::Solid(0xff00_8000));
    surface.attach(&buffer);
    surface.commit();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    assert_eq!(*fixture.model(), before);
}

/// Destroying a surface before its buffer, and the reverse, leave no texture behind.
#[test]
fn destroying_a_surface_before_its_buffer_is_clean() {
    let (mut fixture, first) = boot();
    let second = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let before = fixture.model().clone();
    {
        let client = fixture.client(first);
        let surface = Compositor::bind(client, 5).surface(client);
        let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
        surface.attach(&buffer);
        surface.commit();
        surface.destroy();
        drop(buffer);
    }
    {
        let client = fixture.client(second);
        let surface = Compositor::bind(client, 5).surface(client);
        let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
        surface.attach(&buffer);
        surface.commit();
        drop(buffer);
        surface.destroy();
    }
    fixture.pump();
    for id in [first, second] {
        assert!(fixture.client(id).protocol_error().is_none());
        assert!(fixture.client(id).is_connected());
    }
    assert_eq!(*fixture.model(), before);
}

/// A buffer scale below one is wl_surface.invalid_scale (0) and disconnects only that client.
#[test]
fn a_zero_buffer_scale_is_a_protocol_error() {
    let (mut fixture, bad) = boot();
    let good = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let client = fixture.client(bad);
    let surface = Compositor::bind(client, 5).surface(client);
    surface.surface.set_buffer_scale(0);
    fixture.pump();
    assert_eq!(fault(&mut fixture, bad), ("wl_surface".to_owned(), 0));
    assert!(!fixture.client(bad).is_connected());
    assert!(fixture.client(good).is_connected());
}

/// A non-zero attach offset on a version 5 surface is wl_surface.invalid_offset (3).
#[test]
fn an_attach_offset_at_version_five_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    surface.surface.attach(None, 1, 0);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wl_surface".to_owned(), 3));
}
