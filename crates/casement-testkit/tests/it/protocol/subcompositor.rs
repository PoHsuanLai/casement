//! `subcompositor` (P0-H1).

use casement_core::geometry::PhysicalSize;
use casement_testclient::surface::subsurface::{Compositor, Subcompositor};
use casement_testclient::{Pattern, shm_buffer};

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

const SIZE: PhysicalSize = PhysicalSize { w: 32, h: 32 };

/// Place_above/place_below order and synchronised commits reach the frame together.
#[test]
fn subsurfaces_stack_and_sync() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let compositor = Compositor::bind(client, 5);
    let subcompositor = Subcompositor::bind(client, 1);
    let parent = compositor.surface(client);
    let first = compositor.surface(client);
    let second = compositor.surface(client);
    let a = subcompositor.subsurface(client, &first, &parent);
    let b = subcompositor.subsurface(client, &second, &parent);
    a.place_above(&second.surface);
    b.place_below(&first.surface);
    a.set_position(10, 10);
    a.set_sync();
    b.set_desync();
    let buffers: Vec<_> = (0..3)
        .map(|_| shm_buffer(client, SIZE, Pattern::Gradient))
        .collect();
    for (surface, buffer) in [&first, &second, &parent].into_iter().zip(&buffers) {
        surface.attach(buffer);
        surface.commit();
    }
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    // The buffers are the only events: one wl_shm format pair per buffer.
    check(
        "subcompositor-tree.txt",
        fixture.client(id),
        &["wl_registry", "wl_display", "wl_shm"],
    );
}

/// Making a surface its own subsurface is the bad_surface error and disconnects only that client.
#[test]
fn a_subsurface_of_itself_is_a_protocol_error() {
    let (mut fixture, bad) = boot();
    let good = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let client = fixture.client(bad);
    let compositor = Compositor::bind(client, 5);
    let subcompositor = Subcompositor::bind(client, 1);
    let surface = compositor.surface(client);
    let _ = subcompositor.subsurface(client, &surface, &surface);
    fixture.pump();
    // wl_subcompositor.error.bad_surface.
    assert_eq!(fault(&mut fixture, bad), ("wl_subcompositor".to_owned(), 0));
    assert!(fixture.client(good).is_connected());
}

/// A sibling that is not one is wl_subsurface.bad_surface.
#[test]
fn restacking_against_a_stranger_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let compositor = Compositor::bind(client, 5);
    let subcompositor = Subcompositor::bind(client, 1);
    let parent = compositor.surface(client);
    let child = compositor.surface(client);
    let stranger = compositor.surface(client);
    let sub = subcompositor.subsurface(client, &child, &parent);
    sub.place_above(&stranger.surface);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wl_subsurface".to_owned(), 0));
}

/// A second subsurface role on the same surface is bad_surface.
#[test]
fn a_surface_cannot_be_a_subsurface_twice() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let compositor = Compositor::bind(client, 5);
    let subcompositor = Subcompositor::bind(client, 1);
    let parent = compositor.surface(client);
    let child = compositor.surface(client);
    let _first = subcompositor.subsurface(client, &child, &parent);
    let _second = subcompositor.subsurface(client, &child, &parent);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wl_subcompositor".to_owned(), 0));
}

/// The parent destroyed first: the child's requests stay valid until it is destroyed too.
#[test]
fn destroying_the_parent_first_is_clean() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let compositor = Compositor::bind(client, 5);
    let subcompositor = Subcompositor::bind(client, 1);
    let parent = compositor.surface(client);
    let child = compositor.surface(client);
    let sub = subcompositor.subsurface(client, &child, &parent);
    parent.destroy();
    sub.destroy();
    child.destroy();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
}
