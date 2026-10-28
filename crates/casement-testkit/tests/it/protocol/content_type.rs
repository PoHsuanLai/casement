//! `content_type` (P0-H1).

use casement_testclient::misc::content_type::{ContentTypes, Type, set};
use casement_testclient::surface::subsurface::Compositor;

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
    if Env::of_process().bless {
        std::fs::write(&path, &actual).expect("write the blessed golden");
        return;
    }
    panic!(
        "{} differs\n--- golden\n{golden}--- actual\n{actual}",
        path.display()
    );
}

/// Set_content_type(video) is read at commit.
///
/// What the compositor stores is not visible through the fixture (it holds the model and the
/// frame, not the protocol parts), so this checks the protocol side: every value is accepted
/// at commit, in any order, with no error; `casement_wayland::content_type`'s unit tests check
/// the stored kind.
#[test]
fn the_content_type_is_stored() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let object = ContentTypes::bind(client).for_surface(client, &surface);
    for kind in [Type::Video, Type::Game, Type::Photo, Type::None] {
        set(&object, kind);
        surface.commit();
    }
    object.destroy();
    surface.commit();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    check(
        "content-type-flow.txt",
        fixture.client(id),
        &["wl_registry", "wl_display"],
    );
}

/// A second content-type object on one surface is
/// wp_content_type_manager_v1.already_constructed (0).
#[test]
fn a_second_content_type_object_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let manager = ContentTypes::bind(client);
    let _first = manager.for_surface(client, &surface);
    let _second = manager.for_surface(client, &surface);
    fixture.pump();
    assert_eq!(
        fault(&mut fixture, id),
        ("wp_content_type_manager_v1".to_owned(), 0)
    );
}
